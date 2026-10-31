//! Optional caller-reported client context; never treated as verified identity.
use crate::features::common::error::{ApiError, ApiResult};
use axum::http::HeaderMap;
use serde_json::{Map, Value};
pub(super) fn from_headers(headers: &HeaderMap) -> ApiResult<Option<Value>> {
    let mut context = Map::new();
    for (header, key) in [
        ("x-hive-agent", "agent"),
        ("x-hive-model", "model"),
        ("x-hive-harness", "harness"),
    ] {
        if headers.get_all(header).iter().count() > 1 {
            return Err(ApiError::bad("invalid_client_context"));
        }
        if let Some(value) = headers.get(header) {
            let value = value
                .to_str()
                .map_err(|_| ApiError::bad("invalid_client_context"))?
                .trim();
            if value.is_empty() || value.len() > 120 || value.chars().any(char::is_control) {
                return Err(ApiError::bad("invalid_client_context"));
            }
            context.insert(key.into(), Value::String(value.into()));
        }
    }
    Ok((!context.is_empty()).then_some(Value::Object(context)))
}
