//! Bounded offline schemas. References and branching validation are deliberately unsupported.
use crate::{
    features::common::error::{ApiError, ApiResult},
    runtime::SandboxLimits,
};
use serde_json::Value;

/// Build a bounded offline validator for an object schema.
/// # Errors
/// Rejects oversized schemas, external references and unsupported branching.
pub fn compile(schema: &Value, limits: &SandboxLimits) -> ApiResult<jsonschema::Validator> {
    if serde_json::to_vec(schema)?.len() > limits.schema_bytes || schema["type"] != "object" {
        return Err(ApiError::bad("bounded_object_schema_required"));
    }
    if schema
        .get("$schema")
        .is_some_and(|uri| uri != "https://json-schema.org/draft/2020-12/schema")
    {
        return Err(ApiError::bad("unsupported_schema_draft"));
    }
    inspect(schema, 0)?;
    jsonschema::draft202012::options()
        .with_pattern_options(
            jsonschema::PatternOptions::regex()
                .size_limit(65_536)
                .dfa_size_limit(65_536),
        )
        .build(schema)
        .map_err(|_| ApiError::bad("invalid_capability_schema"))
}
fn inspect(value: &Value, depth: usize) -> ApiResult<()> {
    if depth > 12 {
        return Err(ApiError::bad("capability_schema_too_deep"));
    }
    match value {
        Value::Object(fields) => {
            for (key, value) in fields {
                if matches!(
                    key.as_str(),
                    "$ref"
                        | "$dynamicRef"
                        | "$recursiveRef"
                        | "$id"
                        | "allOf"
                        | "anyOf"
                        | "oneOf"
                        | "not"
                        | "if"
                        | "then"
                        | "else"
                        | "dependentSchemas"
                ) {
                    return Err(ApiError::bad("unsupported_capability_schema_feature"));
                }
                inspect(value, depth + 1)?;
            }
        }
        Value::Array(values) => {
            for value in values {
                inspect(value, depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}
