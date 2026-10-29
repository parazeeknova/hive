//! CPU/memory bounded JSON-in/JSON-out WebAssembly capabilities.
mod data;
mod engine;
mod schemas;
use crate::{
    features::common::error::{ApiError, ApiResult},
    runtime::SandboxLimits,
};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

pub struct Sandbox {
    limits: SandboxLimits,
    slots: Arc<tokio::sync::Semaphore>,
}
impl Sandbox {
    #[must_use]
    pub fn new(limits: SandboxLimits) -> Self {
        Self {
            slots: Arc::new(tokio::sync::Semaphore::new(limits.concurrency)),
            limits,
        }
    }
    async fn job<T: Send + 'static>(
        &self,
        work: impl FnOnce(SandboxLimits) -> ApiResult<T> + Send + 'static,
    ) -> ApiResult<T> {
        let permit =
            tokio::time::timeout(Duration::from_secs(10), self.slots.clone().acquire_owned())
                .await
                .map_err(|_| ApiError::unavailable("sandbox_busy"))?
                .map_err(|_| ApiError::unavailable("sandbox_busy"))?;
        let limits = self.limits.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            work(limits)
        })
        .await
        .map_err(|_| ApiError::unavailable("sandbox_task_failed"))?
    }
    /// Validate the module and required ABI without running creator functions.
    /// # Errors
    /// Rejects invalid modules, host imports, unsupported ABIs and exhausted capacity.
    pub async fn verify(
        &self,
        bytes: Vec<u8>,
        input_schema: Value,
        output_schema: Value,
    ) -> ApiResult<()> {
        self.job(move |limits| {
            schemas::compile(&input_schema, &limits)?;
            schemas::compile(&output_schema, &limits)?;
            engine::verify(&engine::engine(), &bytes, &limits)
        })
        .await
    }
    /// Validate bounded input off the asynchronous request executor.
    /// # Errors
    /// Rejects oversized, structurally expensive or schema-incompatible data and exhausted capacity.
    pub async fn validate_input(&self, input: Value, schema: Value) -> ApiResult<()> {
        self.job(move |limits| validate_input(&input, &schema, &limits))
            .await
    }
    /// Run pure Wasm with metered instructions and no ambient capabilities.
    /// # Errors
    /// Rejects invalid input/output, trapped modules and exhausted capacity.
    pub async fn execute(
        &self,
        bytes: Vec<u8>,
        input: Value,
        input_schema: Value,
        output_schema: Value,
    ) -> ApiResult<Value> {
        self.job(move |limits| {
            validate_input(&input, &input_schema, &limits)?;
            let output = engine::execute(&engine::engine(), &bytes, &input, &limits)?;
            data::validate(&output, &limits, limits.output_bytes)?;
            if !schemas::compile(&output_schema, &limits)?.is_valid(&output) {
                return Err(ApiError::bad("capability_output_schema_mismatch"));
            }
            Ok(output)
        })
        .await
    }
}
fn validate_input(input: &Value, schema: &Value, limits: &SandboxLimits) -> ApiResult<()> {
    data::validate(input, limits, limits.input_bytes)?;
    if !schemas::compile(schema, limits)?.is_valid(input) {
        return Err(ApiError::bad("capability_input_schema_mismatch"));
    }
    Ok(())
}
#[cfg(test)]
mod tests;
