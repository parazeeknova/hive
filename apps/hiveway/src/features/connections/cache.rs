//! Result cache for read-only connected tools.
//!
//! Keys are `sha256(partition, tool, canonical arguments)` under the connection.
//! The partition is empty for owner-private connections (one principal) and for
//! shared integrations the operator marked `public`; every other shared result
//! is partitioned by caller, so one account's answer is never served to another.
//! Entries expire by TTL and are purged whenever the connection's tool snapshot
//! changes, so an upgraded upstream never serves results from its old version.
use super::statement;
use crate::{
    features::common::{error::ApiResult, model::now},
    state::AppState,
};
use sea_orm::ConnectionTrait;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Canonicalize a JSON value so semantically identical arguments hash identically.
pub(super) fn canonical_args(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object({
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            entries
                .into_iter()
                .map(|(key, item)| (key.clone(), canonical_args(item)))
                .collect()
        }),
        Value::Array(items) => Value::Array(items.iter().map(canonical_args).collect()),
        _ => value.clone(),
    }
}
pub(super) fn key(partition: &str, tool: &str, args: &Value) -> String {
    let mut hasher = Sha256::new();
    for part in [partition, tool, &canonical_args(args).to_string()] {
        hasher.update(part.len().to_be_bytes());
        hasher.update(part.as_bytes());
    }
    hex::encode(hasher.finalize())
}
/// Whether the snapshotted upstream tool is safe to cache.
pub(super) fn tool_is_read_only(tools: &Value, tool: &str) -> bool {
    tools.as_array().is_some_and(|list| {
        list.iter()
            .any(|item| item["name"] == tool && item["annotations"]["readOnlyHint"] == true)
    })
}
/// Fetch a live entry and mark the hit so cache value stays observable.
pub(super) async fn lookup(
    state: &AppState,
    connection: Uuid,
    tool: &str,
    key: &str,
) -> ApiResult<Option<Value>> {
    let row = state
        .postgres
        .query_one_raw(statement(
            "UPDATE mcp_cache SET hits=hits+1 WHERE connection_id=$1 AND tool=$2 AND args_hash=$3 AND expires_at>$4 RETURNING result, created_at",
            vec![connection.into(), tool.into(), key.into(), now().into()],
        ))
        .await?;
    let Some(row) = row else { return Ok(None) };
    let mut result: Value = row.try_get("", "result")?;
    let created: i64 = row.try_get("", "created_at")?;
    if result.is_object() {
        result["_meta"]["dev.hive/cache"] =
            json!({"hit":true,"age_seconds":(now()-created).max(0)});
    }
    Ok(Some(result))
}
/// Best-effort insert for small, successful results.
pub(super) async fn store(
    state: &AppState,
    connection: Uuid,
    tool: &str,
    key: &str,
    result: &Value,
) {
    let cache = &state.settings.runtime.cache;
    if cache.connected_tool_ttl_seconds == 0
        || result.get("isError") == Some(&Value::Bool(true))
        || result.to_string().len() > cache.connected_tool_max_bytes
    {
        return;
    }
    let at = now();
    let outcome = state
        .postgres
        .execute_raw(statement(
            "INSERT INTO mcp_cache(connection_id,tool,args_hash,result,expires_at,created_at,hits) VALUES($1,$2,$3,$4,$5,$6,0) ON CONFLICT (connection_id,tool,args_hash) DO UPDATE SET result=EXCLUDED.result,expires_at=EXCLUDED.expires_at,created_at=EXCLUDED.created_at",
            vec![
                connection.into(),
                tool.into(),
                key.into(),
                result.clone().into(),
                (at + i64::try_from(cache.connected_tool_ttl_seconds).unwrap_or(60)).into(),
                at.into(),
            ],
        ))
        .await;
    if let Err(error) = outcome {
        tracing::warn!(%error, "Connected tool cache write failed");
    }
}
/// Drop everything cached for a connection whose snapshot changed.
pub(super) async fn purge(txn: &impl ConnectionTrait, connection: Uuid) -> ApiResult<()> {
    txn.execute_raw(statement(
        "DELETE FROM mcp_cache WHERE connection_id=$1",
        vec![connection.into()],
    ))
    .await?;
    Ok(())
}
/// Drop expired entries.
/// # Errors
/// Returns database errors.
pub(super) async fn cleanup(state: &AppState) -> ApiResult<()> {
    state
        .postgres
        .execute_raw(statement(
            "DELETE FROM mcp_cache WHERE expires_at<$1",
            vec![now().into()],
        ))
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_keys_ignore_argument_order_but_not_partition() {
        let left = json!({"b": 2, "a": {"y": [1], "x": 0}});
        let right = json!({"a": {"x": 0, "y": [1]}, "b": 2});
        assert_eq!(canonical_args(&left), canonical_args(&right));
        assert_eq!(key("", "search", &left), key("", "search", &right));
        assert_ne!(
            key("github:1", "search", &left),
            key("github:2", "search", &left)
        );
        assert_ne!(key("", "search", &left), key("", "other", &left));
        // Length-prefixing prevents ambiguous concatenations.
        assert_ne!(key("ab", "c", &json!({})), key("a", "bc", &json!({})));
    }
    #[test]
    fn cache_only_applies_to_read_only_tools() {
        let tools = json!([
            {"name": "search", "annotations": {"readOnlyHint": true}},
            {"name": "write", "annotations": {"readOnlyHint": false}},
            {"name": "legacy"}
        ]);
        assert!(tool_is_read_only(&tools, "search"));
        assert!(!tool_is_read_only(&tools, "write"));
        assert!(!tool_is_read_only(&tools, "legacy"));
        assert!(!tool_is_read_only(&tools, "missing"));
    }
}
