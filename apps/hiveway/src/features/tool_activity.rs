//! Account-scoped tool execution metadata. Arguments, results and secrets are never persisted here.
use crate::{
    features::common::{
        error::ApiResult,
        model::{Credential, now},
    },
    state::AppState,
};
use axum::http::HeaderMap;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::{Value, json};
use std::time::Instant;
use uuid::Uuid;

pub const RUN_HEADER: &str = "x-hive-internal-run";

pub struct Call {
    pub id: Uuid,
    started: Instant,
    record: bool,
}
fn sql(query: &str, values: Vec<sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, query, values)
}
fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
/// Lifecycle handles supplied by gateway routing, never inferred from connector content.
#[derive(Clone, Copy)]
pub enum Lifecycle {
    Task(Uuid),
    Workflow { id: Uuid, pending: bool },
}
fn status(result: &Value, error: Option<&str>, lifecycle: Option<Lifecycle>) -> &'static str {
    if matches!(
        lifecycle,
        Some(Lifecycle::Task(_) | Lifecycle::Workflow { pending: true, .. })
    ) {
        "accepted"
    } else if error.is_some() || failed(result) {
        "failed"
    } else {
        "succeeded"
    }
}
/// MCP semantic failure is independent of HTTP status and arbitrary upstream data fields.
#[must_use]
pub fn failed(value: &Value) -> bool {
    value["isError"] == true
}
impl Call {
    pub async fn start(
        state: &AppState,
        credential: &Credential,
        tool: &str,
        headers: &HeaderMap,
    ) -> Self {
        let call = Self {
            id: Uuid::new_v4(),
            started: Instant::now(),
            record: !matches!(
                tool,
                "poll_mcp"
                    | "poll_mcp_batch"
                    | "complete_mcp"
                    | "progress_mcp"
                    | "elicit_mcp"
                    | "await_input_mcp"
            ),
        };
        if !call.record {
            return call;
        }
        let tool = if identifier(tool) {
            tool
        } else {
            "invalid_tool"
        };
        let provider = tool
            .strip_prefix("mcp_")
            .and_then(|s| s.split_once("__").map(|(slug, _)| slug));
        let context: serde_json::Map<String, Value> = [
            ("agent", "x-hive-agent"),
            ("model", "x-hive-model"),
            ("harness", "x-hive-harness"),
        ]
        .into_iter()
        .filter_map(|(key, header)| {
            headers
                .get(header)
                .and_then(|v| v.to_str().ok())
                .filter(|v| v.len() <= 120 && !v.chars().any(char::is_control))
                .map(|v| (key.to_owned(), json!(v)))
        })
        .collect();
        let run: Option<Uuid> = headers
            .get(RUN_HEADER)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok());
        let deadline =
            state.settings.runtime.connectors.job_ttl_seconds.max(
                i64::try_from(state.settings.runtime.workflows.run_seconds).unwrap_or(i64::MAX),
            );
        let outcome = state.postgres.execute_raw(sql(
            "INSERT INTO mcp_activity(id,owner,credential_id,tool,provider,client_context,run_id,state,created_at,deadline,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7,'running',$8,$9,$10)",
            vec![call.id.into(), credential.wallet.clone().into(), credential.id.into(), tool.into(), provider.into(), Value::Object(context).into(), run.into(), now().into(), now().saturating_add(deadline).into(), now().saturating_add(state.settings.runtime.workflows.run_retention_seconds).into()],
        )).await;
        if let Err(error) = outcome {
            tracing::error!(call_id = %call.id, %error, "MCP activity could not be started");
        }
        call
    }
    pub async fn finish(
        self,
        state: &AppState,
        result: &Value,
        error: Option<&'static str>,
        lifecycle: Option<Lifecycle>,
    ) {
        if !self.record {
            return;
        }
        let (task, run) = match lifecycle {
            Some(Lifecycle::Task(id)) => (Some(id), None),
            Some(Lifecycle::Workflow { id, .. }) => (None, Some(id)),
            None => (None, None),
        };
        let failed = error.is_some() || failed(result);
        let status = status(result, error, lifecycle);
        let pending = status == "accepted";
        // Only gateway-generated codes enter telemetry. Upstream error strings may
        // contain credentials or document contents, even when they look like identifiers.
        let code = error.or(if failed {
            Some("tool_execution_failed")
        } else {
            None
        });
        let elapsed = i64::try_from(self.started.elapsed().as_millis()).unwrap_or(i64::MAX);
        let outcome = state.postgres.execute_raw(sql(
            "UPDATE mcp_activity SET state=$2,error_code=$3,execution_ms=$4,completed_at=$5,run_id=COALESCE(run_id,$6),task_id=$7 WHERE id=$1",
            vec![self.id.into(), status.into(), code.into(), elapsed.into(), (!pending).then(now).into(), run.into(), task.into()],
        )).await;
        if let Err(error) = outcome {
            tracing::error!(call_id = %self.id, %error, "MCP activity outcome could not be recorded");
        }
        if failed {
            tracing::warn!(call_id = %self.id, state = status, execution_ms = elapsed, error_code = code, "MCP tool call finished");
        } else {
            tracing::info!(call_id = %self.id, state = status, execution_ms = elapsed, "MCP tool call finished");
        }
    }
}
/// Reconcile deferred jobs and runs without depending on clients polling them; expire abandoned calls.
/// # Errors
/// Returns database failures.
pub async fn cleanup(state: &AppState) -> ApiResult<()> {
    state.postgres.execute_raw(sql(
        "UPDATE mcp_activity a SET state=CASE WHEN r.status='completed' THEN 'succeeded' ELSE 'failed' END,error_code=CASE WHEN r.status='completed' THEN NULL ELSE 'workflow_execution_failed' END,completed_at=r.updated_at,execution_ms=GREATEST(0,r.updated_at-a.created_at)*1000 FROM workflow_runs r WHERE a.state='accepted' AND a.task_id IS NULL AND a.run_id=r.id AND a.owner=r.owner AND r.status<>'running'",
        vec![],
    )).await?;
    state.postgres.execute_raw(sql(
        "UPDATE mcp_activity a SET state=CASE WHEN j.status='completed' AND COALESCE(j.result->>'isError','false')<>'true' THEN 'succeeded' ELSE 'failed' END,error_code=CASE WHEN j.status='completed' AND COALESCE(j.result->>'isError','false')<>'true' THEN NULL ELSE 'connected_tool_failed' END,completed_at=j.updated_at,execution_ms=GREATEST(0,j.updated_at-a.created_at)*1000 FROM mcp_jobs j WHERE a.state='accepted' AND a.task_id=j.id AND a.owner=j.caller AND j.status IN ('completed','failed','cancelled')",
        vec![],
    )).await?;
    state.postgres.execute_raw(sql("UPDATE mcp_activity SET state='interrupted',error_code='execution_outcome_unknown',completed_at=$1 WHERE state IN ('running','accepted') AND deadline<$1", vec![now().into()])).await?;
    state
        .postgres
        .execute_raw(sql(
            "DELETE FROM mcp_activity WHERE expires_at<$1",
            vec![now().into()],
        ))
        .await?;
    Ok(())
}

/// Read one bounded page, keeping scoped API keys limited to their own calls.
/// # Errors
/// Returns database failures.
pub async fn list(
    state: &AppState,
    credential: &Credential,
    account: bool,
    before: Option<(i64, Uuid)>,
) -> ApiResult<Vec<Value>> {
    let rows = state.postgres.query_all_raw(sql(
        "SELECT a.*,c.name AS agent_name FROM mcp_activity a JOIN agent_credentials c ON c.id=a.credential_id WHERE a.owner=$1 AND ($2 OR a.credential_id=$3) AND ($4::bigint IS NULL OR (a.created_at,a.id)<($4,$5)) ORDER BY a.created_at DESC,a.id DESC LIMIT 101",
        vec![credential.wallet.clone().into(), account.into(), credential.id.into(), before.map(|v|v.0).into(), before.map(|v|v.1).into()],
    )).await?;
    rows.into_iter().map(|r| Ok(json!({
        "id":r.try_get::<Uuid>("","id")?,"tool":r.try_get::<String>("","tool")?,"provider":r.try_get::<Option<String>>("","provider")?,
        "agent_name":r.try_get::<String>("","agent_name")?,"client_context":r.try_get::<Value>("","client_context")?,
        "run_id":r.try_get::<Option<Uuid>>("","run_id")?,"task_id":r.try_get::<Option<Uuid>>("","task_id")?,
        "state":r.try_get::<String>("","state")?,"error_code":r.try_get::<Option<String>>("","error_code")?,
        "execution_ms":r.try_get::<Option<i64>>("","execution_ms")?,"created_at":r.try_get::<i64>("","created_at")?,"completed_at":r.try_get::<Option<i64>>("","completed_at")?,
    }))).collect()
}
/// Counts are separate from paid invocation counts: free calls are not payment receipts.
/// # Errors
/// Returns database failures.
pub async fn counts(state: &AppState, credential: &Credential, account: bool) -> ApiResult<Value> {
    let r=state.postgres.query_one_raw(sql(
        "SELECT count(*) AS total,count(*) FILTER (WHERE state='succeeded') AS succeeded,count(*) FILTER (WHERE state IN ('failed','interrupted')) AS failed FROM mcp_activity WHERE owner=$1 AND ($2 OR credential_id=$3)",
        vec![credential.wallet.clone().into(),account.into(),credential.id.into()],
    )).await?.ok_or_else(crate::features::common::error::ApiError::forbidden)?;
    Ok(
        json!({"total":r.try_get::<i64>("","total")?,"succeeded":r.try_get::<i64>("","succeeded")?,"failed":r.try_get::<i64>("","failed")?}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn application_data_cannot_change_activity_outcomes() {
        let id = Uuid::new_v4();
        let envelope = json!({"isError":false,"structuredContent":{"isError":true,"task_id":id,"task":{"taskId":id,"status":"queued"},"run_id":id,"status":"running"},"_meta":{"dev.hive/task":{"taskId":id,"status":"working"}}});
        assert_eq!(status(&envelope, None, None), "succeeded");
        assert_eq!(status(&envelope, Some("gateway_error"), None), "failed");
        assert_eq!(status(&json!({"isError":true}), None, None), "failed");
        assert_eq!(
            status(&envelope, None, Some(Lifecycle::Task(id))),
            "accepted"
        );
        assert_eq!(
            status(
                &envelope,
                None,
                Some(Lifecycle::Workflow { id, pending: true })
            ),
            "accepted"
        );
        assert_eq!(
            status(
                &envelope,
                None,
                Some(Lifecycle::Workflow { id, pending: false })
            ),
            "succeeded"
        );
    }
    #[test]
    fn distinguishes_tool_errors_from_upstream_status_fields() {
        assert!(!failed(&json!({"status":"failed"})));
        assert!(failed(&json!({"isError":true})));
        assert!(!failed(&json!({"status":"running"})));
        assert!(!identifier("upstream error with secrets"));
        assert!(!identifier(&"x".repeat(129)));
    }
}
