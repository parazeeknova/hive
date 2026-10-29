//! CPU/memory bounded JSON-in/JSON-out WebAssembly capabilities.
mod engine;
pub mod schemas;
use crate::{
    features::common::error::{ApiError, ApiResult},
    runtime::SandboxLimits,
};
use serde_json::Value;
use std::sync::Arc;

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
    /// Validate the module and required ABI without running creator functions.
    /// # Errors
    /// Rejects invalid modules, host imports, unsupported ABIs and exhausted capacity.
    pub async fn verify(
        &self,
        bytes: Vec<u8>,
        input_schema: Value,
        output_schema: Value,
    ) -> ApiResult<()> {
        let permit = self
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| ApiError::unavailable("sandbox_busy"))?;
        let limits = self.limits.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            {
                schemas::compile(&input_schema, &limits)?;
                schemas::compile(&output_schema, &limits)?;
                engine::verify(&engine::engine(), &bytes, &limits)
            }
        })
        .await
        .map_err(|_| ApiError::unavailable("sandbox_task_failed"))?
    }
    /// Run pure Wasm with metered instructions and no ambient capabilities.
    /// # Errors
    /// Rejects invalid input/output, trapped modules and exhausted capacity.
    pub async fn execute(
        &self,
        bytes: Vec<u8>,
        input: Value,
        output_schema: Value,
    ) -> ApiResult<Value> {
        let permit = self
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| ApiError::unavailable("sandbox_busy"))?;
        let limits = self.limits.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let output = engine::execute(&engine::engine(), &bytes, &input, &limits)?;
            if !schemas::compile(&output_schema, &limits)?.is_valid(&output) {
                return Err(ApiError::bad("capability_output_schema_mismatch"));
            }
            Ok(output)
        })
        .await
        .map_err(|_| ApiError::unavailable("sandbox_task_failed"))?
    }
}
#[cfg(test)]
mod tests;
