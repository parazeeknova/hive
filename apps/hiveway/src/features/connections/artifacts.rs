//! Explicit caller-bound MCP artifacts. Wasm retains its pure sandbox ABI.
use crate::{
    features::common::{
        error::{ApiError, ApiResult},
        model::Credential,
    },
    state::AppState,
};
use sea_orm::{ConnectionTrait, DbBackend, EntityTrait, Statement};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    connection_id: Uuid,
    arguments: Value,
}
/// Validate explicit connection binding.
/// # Errors
/// Rejects malformed or oversized arguments.
pub fn validate(input: &Value) -> ApiResult<()> {
    let binding: Binding = serde_json::from_value(input.clone())
        .map_err(|_| ApiError::bad("connection_binding_required"))?;
    if !binding.arguments.is_object() || input.to_string().len() > 131_072 {
        return Err(ApiError::bad("invalid_mcp_arguments"));
    }
    Ok(())
}
/// Validate a bound upstream tool name.
/// # Errors
/// Rejects unsafe names.
pub fn tool_name(name: &str) -> ApiResult<()> {
    if !super::valid_name(name) {
        return Err(ApiError::bad("invalid_mcp_tool_name"));
    }
    Ok(())
}
/// Execute with the invocation's credential, never the publisher's identity.
/// # Errors
/// Rejects unknown credentials, foreign connections, upstream errors and repeated attempts.
pub async fn execute(
    state: &AppState,
    definition: &Value,
    input: &Value,
    credential_id: Uuid,
    invocation_id: Uuid,
) -> ApiResult<Value> {
    let credential = crate::entities::agent_credentials::Entity::find_by_id(credential_id)
        .into_model::<Credential>()
        .one(&state.postgres)
        .await?
        .ok_or_else(ApiError::forbidden)?;
    if credential.revoked_at.is_some()
        || credential.expires_at <= crate::features::common::model::now()
        || !credential.scopes.iter().any(|s| s == "capabilities:invoke")
    {
        return Err(ApiError::forbidden());
    }
    let names = if let Some(names) = definition["mcp_tools"].as_array() {
        names.clone()
    } else {
        vec![definition["tool_name"].clone()]
    };
    let bindings = if definition["mcp_tools"].is_array() {
        input["steps"]
            .as_array()
            .cloned()
            .ok_or_else(|| ApiError::bad("mcp_step_bindings_required"))?
    } else {
        vec![input.clone()]
    };
    if names.is_empty() || names.len() > 4 || bindings.len() != names.len() {
        return Err(ApiError::bad("invalid_mcp_steps"));
    }
    let mut results = Vec::new();
    for (index, (name, binding)) in names.iter().zip(&bindings).enumerate() {
        validate(binding)?;
        let binding: Binding = serde_json::from_value(binding.clone())?;
        let name = name
            .as_str()
            .ok_or_else(|| ApiError::bad("invalid_mcp_tool_name"))?;
        tool_name(name)?;
        let inserted=state.postgres.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO mcp_artifact_attempts(invocation_id,step) VALUES($1,$2) ON CONFLICT DO NOTHING",vec![invocation_id.into(),i32::try_from(index).map_err(|_|ApiError::bad("invalid_mcp_step"))?.into()])).await?;
        if inserted.rows_affected() != 1 {
            return Err(ApiError::bad(
                "mcp_artifact_already_dispatched_outcome_may_be_unknown",
            ));
        }
        let result = super::invoke(
            state,
            &credential,
            &format!("external_{}_{}", binding.connection_id.simple(), name),
            binding.arguments,
        )
        .await?;
        if result["isError"] == true {
            return Err(ApiError::unavailable("connected_mcp_execution_failed"));
        }
        results.push(result);
    }
    Ok(json!({"steps":results}))
}

#[must_use]
pub fn requires_scope(cap: &crate::features::common::model::Capability) -> bool {
    cap.definition["mcp_tools"].is_array()
        || matches!(
            cap.runtime.as_str(),
            "wasm" | "repository_scaffold" | "connected_mcp"
        )
}
