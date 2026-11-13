//! Workflow execution.
//!
//! A run is persisted (`workflow_runs`) and executed in a background task, so
//! it survives the caller disconnecting; `run_workflow` waits briefly and then
//! returns a run handle. Steps whose dependencies are done run concurrently.
//! Every step is executed at most once: a failed or uncertain step is reported,
//! never retried automatically. Runs that outlive their deadline (for example
//! across a restart) are marked interrupted.
use super::{Definition, OnError, Plan, limits, statement, template};
use crate::{
    features::{
        common::{
            error::{ApiError, ApiResult},
            model::{Credential, now},
        },
        realtime,
    },
    state::AppState,
};
use axum::http::{HeaderMap, HeaderValue};
use sea_orm::ConnectionTrait;
use serde_json::{Map, Value, json};
use std::{
    sync::{
        Arc, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tracing::Instrument;
use uuid::Uuid;

fn run_key(id: Uuid) -> String {
    format!("run:{}", id.simple())
}
fn permits(state: &AppState) -> Arc<tokio::sync::Semaphore> {
    static PERMITS: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();
    PERMITS
        .get_or_init(|| Arc::new(tokio::sync::Semaphore::new(limits(state).runs_per_process)))
        .clone()
}
/// Nesting depth of the current call, set by an enclosing workflow.
#[must_use]
pub fn depth(headers: &HeaderMap) -> usize {
    headers
        .get(super::super::DEPTH_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}
fn char_prefix(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_owned();
    }
    let mut end = limit;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}
/// What later steps see of one step: `ok`, `text`, `json`, `result` (and `items`).
#[must_use]
pub fn view(result: &Value, limit: usize) -> Value {
    let failed = result["isError"] == true;
    let text = match result["content"].as_array() {
        Some(blocks) => blocks
            .iter()
            .filter_map(|b| b["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        None => template::text(result),
    };
    let structured = result
        .get("structuredContent")
        .filter(|v| v.is_object() || v.is_array())
        .cloned()
        .map(|value| {
            // FastMCP may wrap JSON returned as text in a `result` field.
            if value.as_object().is_some_and(|map| map.len() == 1)
                && let Some(text) = value["result"].as_str()
                && let Ok(parsed) = serde_json::from_str::<Value>(text)
                && (parsed.is_object() || parsed.is_array())
            {
                return parsed;
            }
            value
        })
        // Many MCP servers return JSON as text; expose it as structured data too.
        .or_else(|| {
            serde_json::from_str::<Value>(text.trim())
                .ok()
                .filter(|v| v.is_object() || v.is_array())
        });
    let json = structured
        .filter(|v| v.to_string().len() <= limit)
        .unwrap_or(Value::Null);
    let full = if result.to_string().len() <= limit {
        result.clone()
    } else {
        Value::Null
    };
    json!({"ok": !failed, "text": char_prefix(&text, limit), "json": json, "result": full})
}
fn failure_view(message: &str) -> Value {
    json!({"ok": false, "text": message, "json": null, "result": null, "error": message})
}
fn preview(view: &Value) -> Value {
    json!({
        "ok": view["ok"],
        "skipped": view.get("skipped"),
        "text": char_prefix(view["text"].as_str().unwrap_or_default(), 2000),
        "items": view.get("items").and_then(Value::as_array).map(Vec::len),
        "error": view.get("error"),
    })
}

/// Shared execution state of one run.
struct Exec {
    state: AppState,
    owner: String,
    headers: HeaderMap,
    plan: Plan,
    inputs: Value,
    run: Option<Uuid>,
    budget: Arc<Budget>,
    execution_id: Uuid,
}
struct Budget {
    calls: AtomicUsize,
    deadline: tokio::time::Instant,
    run: Uuid,
}
tokio::task_local! {
    static BUDGET: Arc<Budget>;
}
impl Exec {
    fn wait(&self) -> Duration {
        let step = Duration::from_secs(limits(&self.state).step_wait_seconds);
        step.min(
            self.budget
                .deadline
                .saturating_duration_since(tokio::time::Instant::now()),
        )
    }
    /// Deterministic idempotency key per run, step and item for catalog capabilities.
    fn idempotency(&self, step: &str, index: Option<usize>) -> String {
        use sha2::{Digest, Sha256};
        let seed = format!(
            "{}:{step}:{}",
            self.execution_id,
            index.unwrap_or(usize::MAX)
        );
        let digest = Sha256::digest(seed.as_bytes());
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&digest[..16]);
        uuid::Builder::from_random_bytes(bytes)
            .into_uuid()
            .to_string()
    }
    async fn call(&self, step: &super::Step, mut args: Value, index: Option<usize>) -> Value {
        let limits = limits(&self.state);
        if self
            .budget
            .calls
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
                (n < limits.max_tool_calls).then_some(n + 1)
            })
            .is_err()
        {
            return failure_view("This run reached its tool call limit.");
        }
        if tokio::time::Instant::now() >= self.budget.deadline {
            return failure_view("The run passed its deadline before this step.");
        }
        if step.tool.starts_with("hive_")
            && let Some(map) = args.as_object_mut()
        {
            map.entry("idempotency_key")
                .or_insert_with(|| json!(self.idempotency(&step.id, index)));
        }
        let mut headers = self.headers.clone();
        if let Ok(value) = HeaderValue::from_str(&(depth(&self.headers) + 1).to_string()) {
            headers.insert(super::super::DEPTH_HEADER, value);
        }
        if let Ok(value) = HeaderValue::from_str(&self.budget.run.to_string()) {
            headers.insert(crate::features::tool_activity::RUN_HEADER, value);
        }
        let wait = self.wait();
        let outcome = tokio::time::timeout(
            (wait + Duration::from_secs(5)).min(
                self.budget
                    .deadline
                    .saturating_duration_since(tokio::time::Instant::now()),
            ),
            super::super::invoke_tool(&self.state, &headers, &step.tool, args, wait),
        )
        .await;
        let failed = !matches!(&outcome, Ok(Ok(v)) if v["isError"] != true);
        super::super::discovery::record_usage(&self.state, &self.owner, &step.tool, failed).await;
        match outcome {
            Ok(Ok(result)) => {
                let limit = limits.step_result_bytes;
                let mut v = view(&result, limit);
                if result["isError"] == true {
                    v["error"] = json!(char_prefix(
                        v["text"].as_str().unwrap_or("Step failed"),
                        500
                    ));
                }
                v
            }
            Ok(Err(error)) => {
                let message = error.detail.as_ref().map_or_else(
                    || setup_hint(&step.tool, error.code),
                    |d| format!("{}: {d}", error.code),
                );
                failure_view(&message)
            }
            Err(_) => failure_view(
                "The step did not finish before its deadline; its outcome is unknown and it was not repeated.",
            ),
        }
    }
    async fn step(&self, index: usize, views: &Map<String, Value>) -> Value {
        let step = &self.plan.definition.steps[index];
        let scope = template::Scope {
            inputs: &self.inputs,
            steps: views,
            item: None,
            index: None,
        };
        if let Some(when) = &step.when
            && step.for_each.is_none()
        {
            match template::render(&json!(when), &scope) {
                Ok(value) if !template::truthy(&value) => {
                    return json!({"ok": true, "skipped": true, "text": "", "json": null, "result": null});
                }
                Ok(_) => {}
                Err(error) => return failure_view(&format!("when: {error}")),
            }
        }
        let Some(source) = &step.for_each else {
            return match template::render(&step.arguments, &scope) {
                Ok(args) => self.call(step, args, None).await,
                Err(error) => failure_view(&format!("arguments: {error}")),
            };
        };
        let items = match template::render(&json!(source), &scope) {
            Ok(Value::Array(items)) => items,
            Ok(Value::Null) => Vec::new(),
            Ok(other) => vec![other],
            Err(error) => return failure_view(&format!("for_each: {error}")),
        };
        let max = limits(&self.state).max_items;
        let truncated = items.len() > max;
        let items: Vec<Value> = items.into_iter().take(max).collect();
        let concurrency = limits(&self.state).step_concurrency;
        let mut results = Vec::with_capacity(items.len());
        for chunk in items
            .chunks(concurrency)
            .enumerate()
            .map(|(n, c)| (n * concurrency, c))
        {
            let (offset, chunk) = chunk;
            // oxlint-style note: chunks run one after another; items in a chunk run together.
            let calls = chunk.iter().enumerate().map(|(i, item)| {
                let position = offset + i;
                let scope = template::Scope {
                    inputs: &self.inputs,
                    steps: views,
                    item: Some(item),
                    index: Some(position),
                };
                let rendered = if let Some(when) = &step.when {
                    template::render(&json!(when), &scope).and_then(|value| {
                        if template::truthy(&value) {
                            template::render(&step.arguments, &scope).map(Some)
                        } else {
                            Ok(None)
                        }
                    })
                } else {
                    template::render(&step.arguments, &scope).map(Some)
                };
                async move {
                    match rendered {
                        Ok(Some(args)) => self.call(step, args, Some(position)).await,
                        Ok(None) => {
                            json!({"ok":true,"skipped":true,"text":"","json":null,"result":null})
                        }
                        Err(error) => failure_view(&format!("arguments: {error}")),
                    }
                }
            });
            results.extend(futures_util::future::join_all(calls).await);
        }
        let ok = results.iter().all(|r| r["ok"] == true);
        let text = results
            .iter()
            .filter_map(|r| r["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        let json_items: Vec<Value> = results.iter().map(|r| r["json"].clone()).collect();
        let mut combined = json!({
            "ok": ok,
            "text": char_prefix(&text, limits(&self.state).step_result_bytes),
            "json": json_items,
            "items": results,
            "result": null,
            "truncated": truncated,
        });
        if !ok {
            combined["error"] = json!("One or more items failed.");
        }
        combined
    }
    async fn record(&self, id: &str, status: &str, view: &Value) {
        let Some(run) = self.run else { return };
        let mut entry = preview(view);
        entry["status"] = json!(status);
        let outcome = self
            .state
            .postgres
            .execute_raw(statement(
                "UPDATE workflow_runs SET steps = steps || jsonb_build_object($2::text, $3::jsonb), tool_calls=$4, updated_at=$5 WHERE id=$1",
                vec![
                    run.into(),
                    id.into(),
                    entry.into(),
                    i32::try_from(self.budget.calls.load(Ordering::SeqCst)).unwrap_or(i32::MAX).into(),
                    now().into(),
                ],
            ))
            .await;
        if let Err(error) = outcome {
            tracing::warn!(%error, "Workflow step progress was not recorded");
        }
        realtime::announce(&self.state, &[run_key(run)]).await;
    }
    async fn cancelled(&self) -> bool {
        let run = self.budget.run;
        self.state
            .postgres
            .query_one_raw(statement(
                "SELECT status FROM workflow_runs WHERE id=$1",
                vec![run.into()],
            ))
            .await
            .ok()
            .flatten()
            .and_then(|row| row.try_get::<String>("", "status").ok())
            .is_some_and(|status| status != "running")
    }
    /// Run every step; returns the step views and the first stopping failure.
    async fn execute(&self) -> (Map<String, Value>, Option<String>) {
        let steps = &self.plan.definition.steps;
        let mut views: Map<String, Value> = Map::new();
        let mut done = vec![false; steps.len()];
        let concurrency = limits(&self.state).step_concurrency;
        loop {
            let ready: Vec<usize> = (0..steps.len())
                .filter(|&i| !done[i] && self.plan.dependencies[i].iter().all(|&d| done[d]))
                .take(concurrency)
                .collect();
            if ready.is_empty() {
                return (views, None);
            }
            if self.cancelled().await {
                return (views, Some("Cancelled at the caller's request.".into()));
            }
            for &i in &ready {
                self.record(&steps[i].id, "running", &json!({"ok":null,"text":""}))
                    .await;
            }
            let snapshot = views.clone();
            let results =
                futures_util::future::join_all(ready.iter().map(|&i| self.step(i, &snapshot)))
                    .await;
            let mut failure = None;
            for (&i, view) in ready.iter().zip(results) {
                let step = &steps[i];
                let status = if view.get("skipped") == Some(&json!(true)) {
                    "skipped"
                } else if view["ok"] == true {
                    "completed"
                } else {
                    "failed"
                };
                self.record(&step.id, status, &view).await;
                if status == "failed" && step.on_error == OnError::Stop && failure.is_none() {
                    failure = Some(format!(
                        "Step `{}` ({}) failed: {}",
                        step.id,
                        step.tool,
                        view["error"]
                            .as_str()
                            .or_else(|| view["text"].as_str())
                            .unwrap_or("error")
                    ));
                }
                views.insert(step.id.clone(), view);
                done[i] = true;
            }
            if failure.is_some() {
                return (views, failure);
            }
        }
    }
    fn output(&self, views: &Map<String, Value>) -> Result<Value, template::TemplateError> {
        let definition = &self.plan.definition;
        let scope = template::Scope {
            inputs: &self.inputs,
            steps: views,
            item: None,
            index: None,
        };
        if let Some(output) = &definition.output {
            return template::render(output, &scope);
        }
        let last = definition
            .steps
            .iter()
            .rev()
            .find_map(|step| views.get(&step.id).filter(|v| v.get("skipped").is_none()))
            .cloned()
            .unwrap_or(Value::Null);
        if last["json"].is_null() {
            Ok(last["text"].clone())
        } else {
            Ok(last["json"].clone())
        }
    }
}
/// Hint for steps whose tool is missing, e.g. a Hive Local tool that is not running.
fn setup_hint(tool: &str, code: &str) -> String {
    let slug = tool
        .strip_prefix("mcp_")
        .and_then(|rest| rest.split_once("__"))
        .map(|(slug, _)| slug);
    let local = slug.and_then(|slug| {
        crate::features::integrations::registry()
            .iter()
            .find(|item| item.slug == slug && item.local.is_some())
    });
    match (code, local) {
        (
            "mcp_connection_not_connected"
            | "mcp_tool_offline_or_unknown"
            | "shared_integration_offline",
            Some(item),
        ) => format!(
            "{code}: {} runs through Hive Local on your computer. Run `hive-local enable {}` and keep `hive-local run` running, then retry.",
            item.name, item.slug
        ),
        _ => code.to_owned(),
    }
}

/// Options for one run.
pub struct Start<'a> {
    pub credential: &'a Credential,
    pub headers: &'a HeaderMap,
    pub plan: Plan,
    pub inputs: Value,
    pub workflow: Option<Uuid>,
    pub wait: Duration,
}
fn result_value(run: &Value) -> Value {
    let failed = run["status"] == "failed" || run["status"] == "interrupted";
    let text = match &run["output"] {
        Value::String(s) => s.clone(),
        Value::Null if run["status"] == "running" => format!(
            "Workflow run {} is still running. Call get_workflow_run with this run_id and wait_seconds to follow it; do not start it again.",
            run["run_id"].as_str().unwrap_or_default()
        ),
        Value::Null => run["error"].as_str().unwrap_or_default().to_owned(),
        other => serde_json::to_string_pretty(other).unwrap_or_default(),
    };
    let text = if failed {
        format!(
            "{}{}",
            run["error"].as_str().unwrap_or("Workflow failed."),
            if text.is_empty() || run["error"].as_str() == Some(text.as_str()) {
                String::new()
            } else {
                format!("\n\n{text}")
            }
        )
    } else {
        text
    };
    json!({"content":[{"type":"text","text":text}],"structuredContent":run,"isError":failed})
}
/// Run nested workflows inline, without a run record of their own.
async fn inline(
    state: &AppState,
    owner: &str,
    headers: &HeaderMap,
    plan: Plan,
    inputs: Value,
) -> Value {
    let exec = Exec {
        state: state.clone(),
        owner: owner.to_owned(),
        headers: headers.clone(),
        plan,
        inputs,
        run: None,
        budget: BUDGET.with(Arc::clone),
        execution_id: Uuid::new_v4(),
    };
    let (views, failure) = exec.execute().await;
    let (output, failure) = match exec.output(&views) {
        Ok(output) => (output, failure),
        Err(error) => (
            Value::Null,
            failure.or_else(|| Some(format!("output: {error}"))),
        ),
    };
    let steps: Map<String, Value> = views.iter().map(|(k, v)| (k.clone(), preview(v))).collect();
    result_value(&json!({
        "status": if failure.is_some() { "failed" } else { "completed" },
        "output": output,
        "error": failure,
        "steps": steps,
        "tool_calls": exec.budget.calls.load(Ordering::SeqCst),
    }))
}
/// Start a run (or execute it inline when nested) and wait up to `wait`.
/// # Errors
/// Rejects callers over their concurrent run limit or nesting depth.
#[allow(clippy::too_many_lines)]
pub async fn start(state: &AppState, start: Start<'_>) -> ApiResult<Value> {
    let limits = limits(state);
    let depth = depth(start.headers);
    if depth > limits.max_depth {
        return Err(ApiError::bad("workflow_nesting_too_deep"));
    }
    let headers = start.headers.clone();
    if depth > 0 {
        return Ok(inline(
            state,
            &start.credential.wallet,
            &headers,
            start.plan,
            start.inputs,
        )
        .await);
    }
    cleanup(state).await?;
    let txn = sea_orm::TransactionTrait::begin(&state.postgres).await?;
    txn.execute_raw(statement(
        "SELECT pg_advisory_xact_lock(hashtextextended($1, 721))",
        vec![start.credential.wallet.clone().into()],
    ))
    .await?;
    let running = txn
        .query_one_raw(statement(
            "SELECT count(*) AS n FROM workflow_runs WHERE owner=$1 AND status='running'",
            vec![start.credential.wallet.clone().into()],
        ))
        .await?
        .map(|row| row.try_get::<i64>("", "n"))
        .transpose()?
        .unwrap_or(0);
    if usize::try_from(running).unwrap_or(usize::MAX) >= limits.runs_per_owner {
        return Err(ApiError::new(
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            "workflow_runs_busy",
        ));
    }
    let permit = permits(state)
        .try_acquire_owned()
        .map_err(|_| ApiError::unavailable("workflow_capacity"))?;
    let id = Uuid::new_v4();
    let at = now();
    txn.execute_raw(statement(
        "INSERT INTO workflow_runs(id,owner,workflow_id,definition,inputs,status,created_at,updated_at,expires_at) VALUES($1,$2,$3,$4,$5,'running',$6,$6,$7)",
        vec![
            id.into(),
            start.credential.wallet.clone().into(),
            start.workflow.into(),
            serde_json::to_value(&start.plan.definition)?.into(),
            start.inputs.clone().into(),
            at.into(),
            (at + limits.run_retention_seconds).into(),
        ],
    )).await?;
    txn.commit().await?;
    let exec = Exec {
        state: state.clone(),
        owner: start.credential.wallet.clone(),
        headers,
        plan: start.plan,
        inputs: start.inputs,
        run: Some(id),
        budget: Arc::new(Budget {
            calls: AtomicUsize::new(0),
            deadline: tokio::time::Instant::now() + Duration::from_secs(limits.run_seconds),
            run: id,
        }),
        execution_id: id,
    };
    let workflow = start.workflow;
    tokio::spawn(async move {
        let _permit = permit;
        let (views, failure) = BUDGET.scope(exec.budget.clone(), exec.execute()).await;
        let (output, failure) = match exec.output(&views) {
            Ok(output) => (output, failure),
            Err(error) => (
                Value::Null,
                failure.or_else(|| Some(format!("output: {error}"))),
            ),
        };
        let learn = workflow.is_none() && failure.is_none() && !exec.cancelled().await;
        let mut stored = output.clone();
        if stored.to_string().len() > limits_of(&exec.state).step_result_bytes {
            stored = json!(char_prefix(
                &template::text(&output),
                limits_of(&exec.state).step_result_bytes
            ));
        }
        let status = if failure.is_some() {
            "failed"
        } else {
            "completed"
        };
        let finished = exec.state.postgres.execute_raw(statement(
            "UPDATE workflow_runs SET status=CASE WHEN status='running' THEN $2 ELSE status END, output=$3, error=COALESCE(error,$4), tool_calls=$5, updated_at=$6 WHERE id=$1",
            vec![
                id.into(),
                status.into(),
                stored.into(),
                failure.clone().into(),
                i32::try_from(exec.budget.calls.load(Ordering::SeqCst)).unwrap_or(i32::MAX).into(),
                now().into(),
            ],
        )).await;
        if let Err(error) = finished {
            tracing::warn!(%error, %id, "Workflow run outcome was not recorded");
        }
        if learn
            && let Err(error) =
                super::patterns::record(&exec.state, &exec.owner, &exec.plan.definition).await
        {
            tracing::warn!(error = error.code, "Workflow pattern was not recorded");
        }
        if let Some(workflow) = workflow {
            let _ = exec
                .state
                .postgres
                .execute_raw(statement(
                    "UPDATE hive_workflows SET runs=runs+1, failures=failures+$2 WHERE id=$1",
                    vec![workflow.into(), i64::from(failure.is_some()).into()],
                ))
                .await;
        }
        realtime::announce(&exec.state, &[run_key(id)]).await;
    }.in_current_span());
    wait(state, &start.credential.wallet, id, start.wait).await
}
fn limits_of(state: &AppState) -> &crate::runtime::Workflows {
    limits(state)
}
async fn load(state: &AppState, owner: &str, id: Uuid) -> ApiResult<Value> {
    let row = state
        .postgres
        .query_one_raw(statement(
            "SELECT id,workflow_id,status,steps,output,error,tool_calls,created_at,updated_at FROM workflow_runs WHERE id=$1 AND owner=$2",
            vec![id.into(), owner.into()],
        ))
        .await?
        .ok_or_else(|| ApiError::new(axum::http::StatusCode::NOT_FOUND, "unknown_workflow_run"))?;
    Ok(json!({
        "run_id": row.try_get::<Uuid>("", "id")?,
        "workflow_id": row.try_get::<Option<Uuid>>("", "workflow_id")?,
        "status": row.try_get::<String>("", "status")?,
        "steps": row.try_get::<Value>("", "steps")?,
        "output": row.try_get::<Option<Value>>("", "output")?,
        "error": row.try_get::<Option<String>>("", "error")?,
        "tool_calls": row.try_get::<i32>("", "tool_calls")?,
        "created_at": row.try_get::<i64>("", "created_at")?,
        "updated_at": row.try_get::<i64>("", "updated_at")?,
    }))
}
/// Wait up to `wait` for a run to finish, then return it as a tool result.
/// # Errors
/// Rejects runs owned by other accounts.
pub async fn wait(state: &AppState, owner: &str, id: Uuid, wait: Duration) -> ApiResult<Value> {
    let deadline = tokio::time::Instant::now() + wait;
    let run = realtime::wait_for(
        &state.realtime,
        &[run_key(id)],
        deadline,
        Duration::from_secs(1),
        || async {
            let run = load(state, owner, id).await?;
            Ok((run["status"] != "running").then_some(run))
        },
    )
    .await?;
    let run = match run {
        Some(run) => run,
        None => load(state, owner, id).await?,
    };
    Ok(result_value(&run))
}
/// Request cancellation; the run stops before its next step.
/// # Errors
/// Rejects unknown runs.
pub async fn cancel(state: &AppState, owner: &str, id: Uuid) -> ApiResult<Value> {
    let updated = state
        .postgres
        .execute_raw(statement(
            "UPDATE workflow_runs SET status='failed', error='Cancelled at the caller''s request.', updated_at=$3 WHERE id=$1 AND owner=$2 AND status='running'",
            vec![id.into(), owner.into(), now().into()],
        ))
        .await?;
    realtime::announce(state, &[run_key(id)]).await;
    Ok(json!({"cancellation_requested": updated.rows_affected() == 1}))
}
/// The definition a finished run used, for `publish_workflow from_run`.
/// # Errors
/// Rejects unknown runs.
pub async fn definition_of(state: &AppState, c: &Credential, run: &str) -> ApiResult<Value> {
    let id: Uuid = run.parse().map_err(|_| ApiError::bad("invalid_run_id"))?;
    state
        .postgres
        .query_one_raw(statement(
            "SELECT definition FROM workflow_runs WHERE id=$1 AND owner=$2",
            vec![id.into(), c.wallet.clone().into()],
        ))
        .await?
        .map(|row| row.try_get::<Value>("", "definition"))
        .transpose()?
        .ok_or_else(|| ApiError::new(axum::http::StatusCode::NOT_FOUND, "unknown_workflow_run"))
}
/// Close runs that outlived their deadline and drop expired run records.
/// # Errors
/// Returns database errors.
pub async fn cleanup(state: &AppState) -> ApiResult<()> {
    let limits = limits(state);
    let at = now();
    let cutoff = at - i64::try_from(limits.run_seconds).unwrap_or(900) - 60;
    state
        .postgres
        .execute_raw(statement(
            "UPDATE workflow_runs SET status='interrupted', error='The run stopped before finishing (for example during a Hive restart). Completed steps are listed; nothing was repeated.', updated_at=$2 WHERE status='running' AND created_at<$1",
            vec![cutoff.into(), at.into()],
        ))
        .await?;
    state
        .postgres
        .execute_raw(statement(
            "DELETE FROM workflow_runs WHERE expires_at<$1",
            vec![at.into()],
        ))
        .await?;
    state
        .postgres
        .execute_raw(statement(
            "DELETE FROM workflow_patterns WHERE last_seen<$1 AND workflow_id IS NULL",
            vec![(at - limits.run_retention_seconds).into()],
        ))
        .await?;
    Ok(())
}
/// Render a definition's arguments that depend only on inputs, without calling tools.
#[must_use]
pub fn dry_run(plan: &Plan, inputs: &Value) -> Value {
    let empty = Map::new();
    let scope = template::Scope {
        inputs,
        steps: &empty,
        item: None,
        index: None,
    };
    let steps: Vec<Value> = plan
        .definition
        .steps
        .iter()
        .zip(&plan.dependencies)
        .map(|(step, needs)| {
            let rendered = if needs.is_empty() && step.for_each.is_none() {
                template::render(&step.arguments, &scope).unwrap_or(Value::Null)
            } else {
                step.arguments.clone()
            };
            json!({
                "id": step.id,
                "tool": step.tool,
                "arguments": rendered,
                "after": needs.iter().map(|&i| plan.definition.steps[i].id.clone()).collect::<Vec<_>>(),
                "when": step.when,
                "for_each": step.for_each,
                "reads_only": super::step_reads_only(&step.tool),
            })
        })
        .collect();
    json!({"dry_run": true, "valid": true, "steps": steps})
}
#[must_use]
pub fn definition_text(definition: &Definition) -> String {
    definition
        .steps
        .iter()
        .map(|s| s.tool.as_str())
        .collect::<Vec<_>>()
        .join(" → ")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn views_expose_text_and_structured_results() {
        let mcp = json!({"content":[{"type":"text","text":"{\"rows\":[[1]]}"}]});
        let v = view(&mcp, 10_000);
        assert_eq!(v["ok"], true);
        assert_eq!(v["json"]["rows"][0][0], 1);
        let failed = view(
            &json!({"content":[{"type":"text","text":"boom"}],"isError":true}),
            10_000,
        );
        assert_eq!(failed["ok"], false);
        let large = json!({"content":[{"type":"text","text":"é".repeat(5000)}]});
        let v = view(&large, 1001);
        assert!(v["text"].as_str().is_some_and(|t| t.len() <= 1004));
        assert!(v["result"].is_null());
        let native = json!({"memories":[{"title":"x"}]});
        assert_eq!(view(&native, 10_000)["json"]["memories"][0]["title"], "x");
        let wrapped = json!({"content":[{"type":"text","text":"{\"rows\":[[42]]}"}],"structuredContent":{"result":"{\"rows\":[[42]]}"}});
        assert_eq!(view(&wrapped, 10_000)["json"]["rows"][0][0], 42);
    }
    #[test]
    fn setup_hints_name_hive_local() {
        assert!(
            setup_hint("mcp_ffmpeg__ffmpeg_trim", "mcp_tool_offline_or_unknown")
                .contains("hive-local enable ffmpeg")
        );
        assert_eq!(setup_hint("recall", "forbidden"), "forbidden");
    }
}
