//! Bound validation work independently of a creator's schema.
use crate::{
    features::common::error::{ApiError, ApiResult},
    runtime::SandboxLimits,
};
use serde_json::Value;

pub(super) fn validate(value: &Value, limits: &SandboxLimits, max_bytes: usize) -> ApiResult<()> {
    if serde_json::to_vec(value)?.len() > max_bytes {
        return Err(ApiError::bad("capability_json_too_large"));
    }
    let mut remaining = limits.json_nodes;
    visit(value, limits, 0, &mut remaining)
}
fn visit(
    value: &Value,
    limits: &SandboxLimits,
    depth: usize,
    remaining: &mut usize,
) -> ApiResult<()> {
    if depth > limits.json_depth || *remaining == 0 {
        return Err(ApiError::bad("capability_json_too_complex"));
    }
    *remaining -= 1;
    match value {
        Value::Array(values) => {
            if values.len() > limits.collection_items {
                return Err(ApiError::bad("capability_json_collection_too_large"));
            }
            for value in values {
                visit(value, limits, depth + 1, remaining)?;
            }
        }
        Value::Object(fields) => {
            if fields.len() > limits.collection_items {
                return Err(ApiError::bad("capability_json_collection_too_large"));
            }
            for value in fields.values() {
                visit(value, limits, depth + 1, remaining)?;
            }
        }
        _ => {}
    }
    Ok(())
}
