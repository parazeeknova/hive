//! Durable connector jobs.
//!
//! A job is one upstream MCP request (`tools/call`, `resources/read` or
//! `prompts/get`) queued for an outbound connector. Its row is the single source
//! of truth for the MCP Tasks extension, Multi Round-Trip input requests and the
//! connector protocol:
//!
//! ```text
//! queued --claim--> working --elicit--> input_required --input--> working
//!    |                 |                      |
//!    +--cancel--> cancelled <------cancel-----+
//!                      +--complete--> completed | failed | cancelled
//! ```
//!
//! Waiters never poll on a fixed short interval: they arm a realtime wakeup and
//! re-read the row when a writer publishes `job:<id>` or `conn:<id>`.
use super::statement;
use crate::{
    api::AdmissionSlot,
    features::{
        common::{
            error::{ApiError, ApiResult},
            model::{Credential, now},
        },
        realtime::{self, connection_key, job_key},
    },
    state::AppState,
};
use sea_orm::{ConnectionTrait, QueryResult};
use serde_json::{Value, json};
use std::time::Duration;
use uuid::Uuid;

/// Elicitation modes the calling MCP client declared for this request.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputModes {
    pub form: bool,
    pub url: bool,
}
impl InputModes {
    /// Read `elicitation` from 2026-07-28 per-request client capabilities.
    /// An empty `elicitation` object implies form mode.
    #[must_use]
    pub fn from_capabilities(capabilities: &Value) -> Self {
        let elicitation = &capabilities["elicitation"];
        if !elicitation.is_object() {
            return Self::default();
        }
        let form = elicitation.get("form").is_some();
        let url = elicitation.get("url").is_some();
        Self {
            form: form || !url,
            url,
        }
    }
    fn names(self) -> Vec<String> {
        let mut names = Vec::new();
        if self.form {
            names.push("form".to_owned());
        }
        if self.url {
            names.push("url".to_owned());
        }
        names
    }
    fn from_names(names: &[String]) -> Self {
        Self {
            form: names.iter().any(|n| n == "form"),
            url: names.iter().any(|n| n == "url"),
        }
    }
    fn covers(self, requests: &Value) -> bool {
        requests.as_object().is_some_and(|map| {
            map.values().all(
                |request| match request["params"]["mode"].as_str().unwrap_or("form") {
                    "url" => self.url,
                    _ => self.form,
                },
            )
        })
    }
}

/// How the gateway may answer a caller while a connected request runs.
#[derive(Clone, Default)]
pub struct CallContext {
    pub modes: InputModes,
    /// The client declared `io.modelcontextprotocol/tasks`.
    pub tasks: bool,
    /// The caller can resume a running job later (task or Hive job handle).
    pub detach: bool,
    /// MRTR retry: the `requestState` echoed by the client.
    pub request_state: Option<String>,
    /// MRTR retry: `inputResponses` keyed like the earlier `inputRequests`.
    pub input_responses: Option<Value>,
    pub slot: Option<AdmissionSlot>,
    /// Overrides `call_wait_seconds`, for workflow steps that run in the background.
    pub wait: Option<Duration>,
}
impl CallContext {
    /// Wait synchronously, never detach and decline any user input request.
    #[must_use]
    pub fn blocking() -> Self {
        Self::default()
    }
    fn park(&self) {
        if let Some(slot) = &self.slot {
            slot.idle();
        }
    }
}

/// The caller-visible state of a connected request.
#[derive(Debug)]
pub enum Outcome {
    /// The upstream result, exactly as the original request would return it.
    Complete(Value),
    /// A JSON-RPC error object for the original request.
    Failed(Value),
    /// The upstream needs user input; the client retries with `inputResponses`.
    InputRequired { job: Uuid, requests: Value },
    /// Still running; `task` is a Tasks-extension `Task` object.
    Running { job: Uuid, task: Value },
}

pub(super) struct NewJob<'a> {
    pub connection: Uuid,
    pub method: &'a str,
    pub request: Value,
    pub caller: &'a str,
    pub cache_key: Option<String>,
    pub modes: InputModes,
    pub shared: bool,
}

const COLUMNS: &str = "id,connection_id,method,request,result,error,status,status_message,input_requests,input_responses,cancel_requested,detached,caller,input_modes,created_at,updated_at,expires_at,claim_expires_at";

struct Job {
    id: Uuid,
    connection_id: Uuid,
    method: String,
    request: Value,
    result: Option<Value>,
    error: Option<Value>,
    status: String,
    status_message: Option<String>,
    input_requests: Option<Value>,
    cancel_requested: bool,
    detached: bool,
    caller: Option<String>,
    input_modes: Vec<String>,
    created_at: i64,
    updated_at: i64,
    expires_at: i64,
    claim_expires_at: Option<i64>,
}
impl Job {
    fn from_row(row: &QueryResult) -> ApiResult<Self> {
        Ok(Self {
            id: row.try_get("", "id")?,
            connection_id: row.try_get("", "connection_id")?,
            method: row.try_get("", "method")?,
            request: row.try_get("", "request")?,
            result: row.try_get("", "result")?,
            error: row.try_get("", "error")?,
            status: row.try_get("", "status")?,
            status_message: row.try_get("", "status_message")?,
            input_requests: row.try_get("", "input_requests")?,
            cancel_requested: row.try_get("", "cancel_requested")?,
            detached: row.try_get("", "detached")?,
            caller: row.try_get("", "caller")?,
            input_modes: row.try_get("", "input_modes")?,
            created_at: row.try_get("", "created_at")?,
            updated_at: row.try_get("", "updated_at")?,
            expires_at: row.try_get("", "expires_at")?,
            claim_expires_at: row.try_get("", "claim_expires_at")?,
        })
    }
    fn terminal(&self) -> bool {
        matches!(self.status.as_str(), "completed" | "failed" | "cancelled")
    }
}

fn limits(state: &AppState) -> &crate::runtime::ConnectorLimits {
    &state.settings.runtime.connectors
}
fn rpc_error(code: i64, message: &str) -> Value {
    json!({"code":code,"message":message})
}
fn iso(seconds: i64) -> String {
    time::OffsetDateTime::from_unix_timestamp(seconds)
        .ok()
        .and_then(|t| {
            t.format(&time::format_description::well_known::Rfc3339)
                .ok()
        })
        .unwrap_or_default()
}

async fn load(state: &AppState, id: Uuid) -> ApiResult<Option<Job>> {
    state
        .postgres
        .query_one_raw(statement(
            &format!("SELECT {COLUMNS} FROM mcp_jobs WHERE id=$1"),
            vec![id.into()],
        ))
        .await?
        .map(|row| Job::from_row(&row))
        .transpose()
}
/// Load a job owned by the calling principal.
async fn load_owned(state: &AppState, c: &Credential, id: Uuid) -> ApiResult<Job> {
    load(state, id)
        .await?
        .filter(|job| job.caller.as_deref() == Some(c.wallet.as_str()) && job.expires_at > now())
        .ok_or_else(|| ApiError::new(axum::http::StatusCode::NOT_FOUND, "unknown_task"))
}

/// Queue a job inside the caller's transaction after enforcing queue bounds.
pub(super) async fn enqueue(
    state: &AppState,
    txn: &sea_orm::DatabaseTransaction,
    job: NewJob<'_>,
) -> ApiResult<Uuid> {
    let limits = limits(state);
    let at = now();
    let count = txn
        .query_one_raw(statement(
            "SELECT count(*) AS count, count(*) FILTER (WHERE caller=$3) AS mine FROM mcp_jobs WHERE connection_id=$1 AND expires_at>$2 AND status IN ('queued','working','input_required')",
            vec![job.connection.into(), at.into(), job.caller.to_owned().into()],
        ))
        .await?
        .ok_or_else(|| ApiError::unavailable("mcp_queue"))?;
    if count.try_get::<i64>("", "count")? >= limits.jobs_per_connection
        || (job.shared && count.try_get::<i64>("", "mine")? >= limits.shared_jobs_per_caller)
    {
        return Err(ApiError::unavailable("mcp_queue_full"));
    }
    let id = Uuid::new_v4();
    txn.execute_raw(statement(
        "INSERT INTO mcp_jobs(id,connection_id,request,expires_at,caller,method,status,claim_expires_at,created_at,updated_at,cache_key,input_modes) VALUES($1,$2,$3,$4,$5,$6,'queued',$7,$8,$8,$9,$10)",
        vec![
            id.into(),
            job.connection.into(),
            job.request.into(),
            (at + limits.job_ttl_seconds).into(),
            job.caller.to_owned().into(),
            job.method.to_owned().into(),
            (at + limits.claim_timeout_seconds).into(),
            at.into(),
            job.cache_key.into(),
            job.modes.names().into(),
        ],
    ))
    .await?;
    Ok(id)
}

/// Fail a queued job no connector claimed in time, so waiters see a definite outcome.
async fn expire_unclaimed(state: &AppState, id: Uuid) -> ApiResult<()> {
    let updated = state
        .postgres
        .execute_raw(statement(
            "UPDATE mcp_jobs SET status='failed', error=$2, updated_at=$3 WHERE id=$1 AND status='queued' AND claim_expires_at<$3",
            vec![
                id.into(),
                rpc_error(-32001, "Connector did not claim the request in time; it was not executed").into(),
                now().into(),
            ],
        ))
        .await?;
    if updated.rows_affected() == 1 {
        realtime::announce(state, &[job_key(id)]).await;
    }
    Ok(())
}

/// Wait for the job, translating its state into what this caller can receive.
pub(super) async fn await_outcome(
    state: &AppState,
    id: Uuid,
    ctx: &CallContext,
) -> ApiResult<Outcome> {
    let limits = limits(state);
    let deadline = tokio::time::Instant::now()
        + ctx
            .wait
            .unwrap_or_else(|| Duration::from_secs(limits.call_wait_seconds));
    ctx.park();
    let outcome = realtime::wait_for(
        &state.realtime,
        &[job_key(id)],
        deadline,
        Duration::from_millis(limits.fallback_poll_ms),
        || async {
            let Some(job) = load(state, id).await? else {
                return Err(ApiError::bad("mcp_connection_removed"));
            };
            if job.status == "queued" && job.claim_expires_at.is_some_and(|at| at < now()) {
                expire_unclaimed(state, id).await?;
                return Ok(None);
            }
            Ok(match job.status.as_str() {
                "completed" | "failed" | "cancelled" => Some(job),
                "input_required"
                    if job
                        .input_requests
                        .as_ref()
                        .is_some_and(|r| ctx.modes.covers(r)) =>
                {
                    Some(job)
                }
                _ => None,
            })
        },
    )
    .await?;
    let Some(job) = outcome else {
        if ctx.detach {
            state
                .postgres
                .execute_raw(statement(
                    "UPDATE mcp_jobs SET detached=true WHERE id=$1",
                    vec![id.into()],
                ))
                .await?;
            let job = load(state, id)
                .await?
                .ok_or_else(|| ApiError::bad("mcp_connection_removed"))?;
            return Ok(Outcome::Running {
                job: id,
                task: task_view(&job, limits.task_poll_interval_ms, false),
            });
        }
        return Err(ApiError::unavailable(
            "mcp_timeout_outcome_unknown_do_not_retry_automatically",
        ));
    };
    if job.status == "input_required" {
        return Ok(Outcome::InputRequired {
            job: id,
            requests: job.input_requests.unwrap_or_else(|| json!({})),
        });
    }
    if !job.detached {
        state
            .postgres
            .execute_raw(statement(
                "DELETE FROM mcp_jobs WHERE id=$1",
                vec![id.into()],
            ))
            .await?;
    }
    Ok(terminal_outcome(job))
}
fn terminal_outcome(job: Job) -> Outcome {
    match job.status.as_str() {
        "completed" => Outcome::Complete(job.result.unwrap_or_else(|| json!({}))),
        "cancelled" => Outcome::Failed(rpc_error(-32800, "Request cancelled")),
        _ => Outcome::Failed(
            job.error
                .unwrap_or_else(|| rpc_error(-32603, "Connected request failed")),
        ),
    }
}

/// MRTR retry: validate the echoed request state and record the client's answers.
/// Returns the input requests that remain unanswered, if any.
pub(super) async fn resume(
    state: &AppState,
    c: &Credential,
    request_state: &str,
    responses: &Value,
    expected: (Uuid, &str),
) -> ApiResult<Option<(Uuid, Value)>> {
    let id: Uuid = request_state
        .parse()
        .map_err(|_| ApiError::bad("invalid_request_state"))?;
    let job = load_owned(state, c, id)
        .await
        .map_err(|_| ApiError::bad("invalid_request_state"))?;
    let (connection, name) = expected;
    let matches = job.connection_id == connection
        && (job.request["name"].as_str() == Some(name)
            || job.request["params"]["uri"].as_str() == Some(name)
            || job.request["params"]["name"].as_str() == Some(name));
    if !matches {
        return Err(ApiError::bad("invalid_request_state"));
    }
    if job.status != "input_required" {
        // Duplicate retry after the answer was accepted: keep waiting on the same job.
        return Ok(Some((id, Value::Null)));
    }
    let remaining = apply_responses(state, &job, responses).await?;
    Ok(Some((id, remaining)))
}
/// Store answers for outstanding input request keys. Unknown keys are ignored.
/// Returns `Null` when every request is answered, otherwise the unanswered subset.
async fn apply_responses(state: &AppState, job: &Job, responses: &Value) -> ApiResult<Value> {
    let requests = job
        .input_requests
        .as_ref()
        .and_then(Value::as_object)
        .ok_or_else(|| ApiError::bad("no_outstanding_input"))?;
    let responses = responses
        .as_object()
        .ok_or_else(|| ApiError::bad("invalid_input_responses"))?;
    if serde_json::to_string(responses)?.len() > limits(state).input_request_bytes {
        return Err(ApiError::bad("input_responses_too_large"));
    }
    let mut answered = serde_json::Map::new();
    let mut remaining = serde_json::Map::new();
    for (key, request) in requests {
        match responses.get(key) {
            Some(response) if response.is_object() => {
                answered.insert(key.clone(), response.clone());
            }
            _ => {
                remaining.insert(key.clone(), request.clone());
            }
        }
    }
    if !remaining.is_empty() {
        return Ok(Value::Object(remaining));
    }
    let updated = state
        .postgres
        .execute_raw(statement(
            "UPDATE mcp_jobs SET status='working', input_requests=NULL, input_responses=$2, updated_at=$3, status_message=NULL WHERE id=$1 AND status='input_required'",
            vec![job.id.into(), Value::Object(answered).into(), now().into()],
        ))
        .await?;
    if updated.rows_affected() == 1 {
        realtime::publish(state, &[job_key(job.id), connection_key(job.connection_id)]).await?;
    }
    Ok(Value::Null)
}

/// A Tasks-extension task object. `detailed` adds the status-specific payload.
fn task_view(job: &Job, poll_interval_ms: u64, detailed: bool) -> Value {
    let status = if job.status == "queued" {
        "working"
    } else {
        job.status.as_str()
    };
    let mut task = json!({
        "taskId": job.id.to_string(),
        "status": status,
        "createdAt": iso(job.created_at),
        "lastUpdatedAt": iso(job.updated_at),
        "ttlMs": (job.expires_at - job.created_at).max(0) * 1000,
        "pollIntervalMs": poll_interval_ms,
    });
    let message = job.status_message.clone().or_else(|| {
        (job.status == "queued")
            .then(|| "Waiting for the connector to claim the request".to_owned())
    });
    if let Some(message) = message {
        task["statusMessage"] = json!(message);
    }
    if job.cancel_requested && !job.terminal() {
        task["statusMessage"] = json!("Cancellation requested");
    }
    if detailed {
        match job.status.as_str() {
            "input_required" => {
                task["inputRequests"] = job.input_requests.clone().unwrap_or_else(|| json!({}));
            }
            "completed" => task["result"] = job.result.clone().unwrap_or_else(|| json!({})),
            "failed" => {
                task["error"] = job
                    .error
                    .clone()
                    .unwrap_or_else(|| rpc_error(-32603, "Connected request failed"));
            }
            _ => {}
        }
    }
    task
}

/// `tasks/get`: the detailed task, including the final result when terminal.
/// # Errors
/// Rejects tasks the caller does not own or that have expired.
pub async fn task(state: &AppState, c: &Credential, task_id: &str) -> ApiResult<(Value, String)> {
    let id = task_id
        .parse()
        .map_err(|_| ApiError::bad("invalid_task_id"))?;
    let job = load_owned(state, c, id).await?;
    if job.status == "queued" && job.claim_expires_at.is_some_and(|at| at < now()) {
        expire_unclaimed(state, id).await?;
        let job = load_owned(state, c, id).await?;
        return Ok((
            task_view(&job, limits(state).task_poll_interval_ms, true),
            job.method,
        ));
    }
    Ok((
        task_view(&job, limits(state).task_poll_interval_ms, true),
        job.method,
    ))
}
/// Wait up to `wait` for a task to change from working, then return it.
/// # Errors
/// Rejects unknown tasks.
pub async fn wait_task(
    state: &AppState,
    c: &Credential,
    task_id: &str,
    wait: Duration,
    slot: Option<&AdmissionSlot>,
) -> ApiResult<(Value, String)> {
    let id: Uuid = task_id
        .parse()
        .map_err(|_| ApiError::bad("invalid_task_id"))?;
    let first = load_owned(state, c, id).await?;
    if !wait.is_zero() && matches!(first.status.as_str(), "queued" | "working") {
        if let Some(slot) = slot {
            slot.idle();
        }
        let started = first.updated_at;
        realtime::wait_for(
            &state.realtime,
            &[job_key(id)],
            tokio::time::Instant::now() + wait,
            Duration::from_millis(limits(state).fallback_poll_ms),
            || async {
                Ok(load(state, id).await?.filter(|job| {
                    !matches!(job.status.as_str(), "queued" | "working")
                        || job.updated_at != started
                }))
            },
        )
        .await?;
    }
    task(state, c, task_id).await
}
/// `tasks/update`: answer outstanding input requests.
/// # Errors
/// Rejects unknown tasks and malformed responses.
pub async fn update_task(
    state: &AppState,
    c: &Credential,
    task_id: &str,
    responses: &Value,
) -> ApiResult<()> {
    let id = task_id
        .parse()
        .map_err(|_| ApiError::bad("invalid_task_id"))?;
    let job = load_owned(state, c, id).await?;
    if job.status != "input_required" {
        // Responses for already-satisfied requests are ignored by specification.
        return Ok(());
    }
    apply_responses(state, &job, responses).await?;
    Ok(())
}
/// `tasks/cancel`: cooperative cancellation.
/// # Errors
/// Rejects unknown tasks.
pub async fn cancel_task(state: &AppState, c: &Credential, task_id: &str) -> ApiResult<()> {
    let id = task_id
        .parse()
        .map_err(|_| ApiError::bad("invalid_task_id"))?;
    let job = load_owned(state, c, id).await?;
    if job.terminal() {
        return Ok(());
    }
    // Unclaimed or blocked-on-user jobs stop immediately; running calls are
    // signalled to the connector, which aborts the upstream request if it can.
    state
        .postgres
        .execute_raw(statement(
            "UPDATE mcp_jobs SET cancel_requested=true, status=CASE WHEN status IN ('queued','input_required') THEN 'cancelled' ELSE status END, input_requests=CASE WHEN status='input_required' THEN NULL ELSE input_requests END, updated_at=$2 WHERE id=$1",
            vec![id.into(), now().into()],
        ))
        .await?;
    realtime::publish(state, &[job_key(id), connection_key(job.connection_id)]).await
}

/// Claim at most one queued job across the given owner connections.
async fn claim(state: &AppState, owner: &str, ids: &[Uuid]) -> ApiResult<Option<Value>> {
    let slots = (2..=ids.len() + 1)
        .map(|index| format!("${index}"))
        .collect::<Vec<_>>()
        .join(",");
    let mut values: Vec<sea_orm::Value> = vec![owner.to_owned().into()];
    values.extend(ids.iter().map(|id| (*id).into()));
    let at = ids.len() + 2;
    let lease_slot = ids.len() + 3;
    values.push(now().into());
    let lease = Uuid::new_v4();
    values.push(lease.into());
    let row = state.postgres.query_one_raw(statement(&format!(
        "UPDATE mcp_jobs SET lease=${lease_slot}, status='working', updated_at=${at} WHERE id=(SELECT j.id FROM mcp_jobs j JOIN mcp_connections c ON c.id=j.connection_id WHERE c.owner=$1 AND NOT ('hosted'=ANY(c.features)) AND j.connection_id IN ({slots}) AND j.lease IS NULL AND j.status='queued' AND j.claim_expires_at>${at} ORDER BY j.created_at FOR UPDATE OF j SKIP LOCKED LIMIT 1) RETURNING id,connection_id,method,request,expires_at"
    ), values)).await?;
    let Some(row) = row else { return Ok(None) };
    let id: Uuid = row.try_get("", "id")?;
    realtime::announce(state, &[job_key(id)]).await;
    let expires: i64 = row.try_get("", "expires_at")?;
    Ok(Some(json!({
        "id": id,
        "connection_id": row.try_get::<Uuid>("", "connection_id")?,
        "lease": lease,
        "method": row.try_get::<String>("", "method")?,
        "request": row.try_get::<Value>("", "request")?,
        "deadline_ms": expires * 1000,
    })))
}

/// `poll_mcp` / `poll_mcp_batch`: heartbeat, then claim or long-poll for a job.
pub(super) async fn poll(
    state: &AppState,
    c: &Credential,
    ids: &[Uuid],
    wait_seconds: u64,
    slot: Option<&AdmissionSlot>,
) -> ApiResult<Value> {
    let limits = limits(state);
    let at = now();
    let slots = (2..=ids.len() + 1)
        .map(|index| format!("${index}"))
        .collect::<Vec<_>>()
        .join(",");
    let mut values: Vec<sea_orm::Value> = vec![c.wallet.clone().into()];
    values.extend(ids.iter().map(|id| (*id).into()));
    values.push(at.into());
    let updated = state
        .postgres
        .execute_raw(statement(
            &format!(
                "UPDATE mcp_connections SET updated_at=${} WHERE owner=$1 AND id IN ({slots})",
                ids.len() + 2
            ),
            values,
        ))
        .await?;
    if updated.rows_affected() != u64::try_from(ids.len()).unwrap_or(u64::MAX) {
        return Err(ApiError::forbidden());
    }
    if let Some(job) = claim(state, &c.wallet, ids).await? {
        return Ok(json!({"job":job}));
    }
    let wait = wait_seconds.min(limits.poll_wait_seconds);
    if wait == 0 {
        return Ok(json!({"job":null}));
    }
    if let Some(slot) = slot {
        slot.idle();
    }
    let keys: Vec<String> = ids.iter().copied().map(connection_key).collect();
    let job = realtime::wait_for(
        &state.realtime,
        &keys,
        tokio::time::Instant::now() + Duration::from_secs(wait),
        Duration::from_millis(limits.fallback_poll_ms.max(1000)),
        || claim(state, &c.wallet, ids),
    )
    .await?;
    Ok(json!({"job":job}))
}

/// Shared lease check for connector-side job operations.
async fn leased(state: &AppState, connection: Uuid, args: &Value) -> ApiResult<Job> {
    let job: Uuid = super::id(args, "job_id")?;
    let lease: Uuid = super::id(args, "lease")?;
    let row = state
        .postgres
        .query_one_raw(statement(
            &format!("SELECT {COLUMNS} FROM mcp_jobs WHERE id=$1 AND connection_id=$2 AND lease=$3 AND expires_at>$4"),
            vec![job.into(), connection.into(), lease.into(), now().into()],
        ))
        .await?
        .ok_or_else(|| ApiError::bad("stale_mcp_job"))?;
    Job::from_row(&row)
}

/// `complete_mcp`: record exactly one outcome for a leased job.
pub(super) async fn complete(state: &AppState, connection: Uuid, args: &Value) -> ApiResult<Value> {
    let limits = limits(state);
    let error = args.get("error").filter(|e| e.is_object()).cloned();
    let result = args.get("result").filter(|r| r.is_object()).cloned();
    if result.is_none() && error.is_none() {
        return Err(ApiError::bad("mcp_result_required"));
    }
    if args.to_string().len() > limits.result_bytes + 4096 {
        return Err(ApiError::bad("mcp_result_limit"));
    }
    let status = if args["cancelled"] == true {
        "cancelled"
    } else if error.is_some() {
        "failed"
    } else {
        "completed"
    };
    let job: Uuid = super::id(args, "job_id")?;
    let row = state.postgres.query_one_raw(statement(
        "UPDATE mcp_jobs SET result=$4, error=$5, status=$6, input_requests=NULL, updated_at=$7 WHERE id=$1 AND connection_id=$2 AND lease=$3 AND result IS NULL AND error IS NULL AND status IN ('working','input_required') AND expires_at>$7 RETURNING cache_key, request",
        vec![
            job.into(),
            connection.into(),
            super::id(args, "lease")?.into(),
            result.clone().into(),
            error.into(),
            status.into(),
            now().into(),
        ],
    )).await?.ok_or_else(|| ApiError::bad("stale_mcp_job"))?;
    if status == "completed"
        && let (Some(key), Some(result)) = (row.try_get::<Option<String>>("", "cache_key")?, result)
    {
        let request: Value = row.try_get("", "request")?;
        super::cache::store(
            state,
            connection,
            request["name"].as_str().unwrap_or_default(),
            &key,
            &result,
        )
        .await;
    }
    realtime::publish(state, &[job_key(job)]).await?;
    Ok(json!({"completed":true}))
}

/// `progress_mcp`: status message heartbeat. Reports pending cancellation.
pub(super) async fn progress(state: &AppState, connection: Uuid, args: &Value) -> ApiResult<Value> {
    let job = leased(state, connection, args).await?;
    let message = args["message"].as_str().map(|m| {
        m.chars()
            .filter(|c| !c.is_control())
            .take(512)
            .collect::<String>()
    });
    let fraction = match (args["progress"].as_f64(), args["total"].as_f64()) {
        (Some(done), Some(total)) if total > 0.0 => Some(format!(
            " ({:.0}%)",
            (done / total * 100.0).clamp(0.0, 100.0)
        )),
        _ => None,
    };
    if let Some(message) = message.map(|m| format!("{m}{}", fraction.unwrap_or_default())) {
        state
            .postgres
            .execute_raw(statement(
                "UPDATE mcp_jobs SET status_message=$2, updated_at=$3 WHERE id=$1 AND status='working'",
                vec![job.id.into(), message.into(), now().into()],
            ))
            .await?;
        realtime::announce(state, &[job_key(job.id)]).await;
    }
    Ok(json!({"cancel_requested":job.cancel_requested}))
}

/// `elicit_mcp`: an upstream server asked the user for input mid-request.
pub(super) async fn elicit(state: &AppState, connection: Uuid, args: &Value) -> ApiResult<Value> {
    let limits = limits(state);
    let job = leased(state, connection, args).await?;
    if job.status != "working" || job.cancel_requested {
        return Ok(json!({"accepted":false,"cancelled":job.cancel_requested}));
    }
    let requests = args["requests"]
        .as_object()
        .filter(|map| !map.is_empty() && map.len() <= limits.input_requests_per_job)
        .ok_or_else(|| ApiError::bad("invalid_input_requests"))?;
    if serde_json::to_string(requests)?.len() > limits.input_request_bytes {
        return Err(ApiError::bad("input_requests_too_large"));
    }
    for (key, request) in requests {
        let mode = request["params"]["mode"].as_str().unwrap_or("form");
        let valid_key = !key.is_empty()
            && key.len() <= 64
            && key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b));
        // Only elicitation is relayed. Sampling and roots are deprecated, and a
        // connector must answer them locally.
        if !valid_key
            || request["method"] != "elicitation/create"
            || !request["params"]["message"].is_string()
            || !matches!(mode, "form" | "url")
            || (mode == "form" && !request["params"]["requestedSchema"].is_object())
            || (mode == "url"
                && !request["params"]["url"]
                    .as_str()
                    .is_some_and(|u| u.starts_with("https://")))
        {
            return Err(ApiError::bad("invalid_input_request"));
        }
    }
    let requests = Value::Object(requests.clone());
    let modes = InputModes::from_names(&job.input_modes);
    if !modes.covers(&requests) {
        // The caller cannot show this request to a user: decline on its behalf
        // instead of leaking an unsupported request or stalling the connector.
        let declined: serde_json::Map<String, Value> = requests
            .as_object()
            .into_iter()
            .flatten()
            .map(|(key, _)| (key.clone(), json!({"action":"decline"})))
            .collect();
        state
            .postgres
            .execute_raw(statement(
                "UPDATE mcp_jobs SET input_responses=$2, updated_at=$3 WHERE id=$1",
                vec![job.id.into(), Value::Object(declined).into(), now().into()],
            ))
            .await?;
        return Ok(json!({"accepted":true,"forwarded":false}));
    }
    let message = requests
        .as_object()
        .and_then(|map| map.values().next())
        .and_then(|request| request["params"]["message"].as_str())
        .map(|m| m.chars().take(512).collect::<String>());
    state
        .postgres
        .execute_raw(statement(
            "UPDATE mcp_jobs SET status='input_required', input_requests=$2, input_responses=NULL, status_message=$3, updated_at=$4 WHERE id=$1 AND status='working'",
            vec![job.id.into(), requests.into(), message.into(), now().into()],
        ))
        .await?;
    realtime::publish(state, &[job_key(job.id)]).await?;
    Ok(json!({"accepted":true,"forwarded":true}))
}

/// `await_input_mcp`: long-poll for the user's answers to an elicitation.
pub(super) async fn await_input(
    state: &AppState,
    connection: Uuid,
    args: &Value,
    slot: Option<&AdmissionSlot>,
) -> ApiResult<Value> {
    let limits = limits(state);
    let job = leased(state, connection, args).await?;
    let wait = args["wait_seconds"]
        .as_u64()
        .unwrap_or(limits.poll_wait_seconds)
        .min(limits.poll_wait_seconds);
    if let Some(slot) = slot {
        slot.idle();
    }
    let take = || async {
        // Take the answers exactly once; a second await sees them as consumed.
        let row = state
            .postgres
            .query_one_raw(statement(
                "WITH pending AS (SELECT id, input_responses FROM mcp_jobs WHERE id=$1 AND input_responses IS NOT NULL FOR UPDATE) UPDATE mcp_jobs j SET input_responses=NULL FROM pending WHERE j.id=pending.id RETURNING pending.input_responses AS responses",
                vec![job.id.into()],
            ))
            .await?;
        if let Some(row) = row {
            return Ok(Some(
                json!({"responses":row.try_get::<Value>("", "responses")?}),
            ));
        }
        let current = load(state, job.id).await?;
        Ok(match current {
            None => Some(json!({"cancelled":true})),
            Some(current) if current.cancel_requested || current.status == "cancelled" => {
                Some(json!({"cancelled":true}))
            }
            _ => None,
        })
    };
    let answer = realtime::wait_for(
        &state.realtime,
        &[job_key(job.id), connection_key(connection)],
        tokio::time::Instant::now() + Duration::from_secs(wait),
        Duration::from_millis(limits.fallback_poll_ms),
        take,
    )
    .await?;
    Ok(answer.unwrap_or_else(|| json!({"pending":true})))
}

/// Retention: fail unclaimed jobs, drop expired ones.
/// # Errors
/// Returns database errors.
pub(super) async fn cleanup(state: &AppState) -> ApiResult<()> {
    let at = now();
    let failed = state
        .postgres
        .query_all_raw(statement(
            "UPDATE mcp_jobs SET status='failed', error=$2, updated_at=$1 WHERE status='queued' AND claim_expires_at<$1 RETURNING id",
            vec![
                at.into(),
                rpc_error(-32001, "Connector did not claim the request in time; it was not executed").into(),
            ],
        ))
        .await?;
    let keys: Vec<String> = failed
        .iter()
        .filter_map(|row| row.try_get::<Uuid>("", "id").ok())
        .map(job_key)
        .collect();
    if !keys.is_empty() {
        realtime::announce(state, &keys).await;
    }
    state
        .postgres
        .execute_raw(statement(
            "DELETE FROM mcp_jobs WHERE expires_at<$1",
            vec![at.into()],
        ))
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn elicitation_capabilities_follow_the_specification() {
        assert_eq!(
            InputModes::from_capabilities(&json!({})),
            InputModes::default()
        );
        assert_eq!(
            InputModes::from_capabilities(&json!({"elicitation":{}})),
            InputModes {
                form: true,
                url: false
            }
        );
        let url_only = InputModes::from_capabilities(&json!({"elicitation":{"url":{}}}));
        assert_eq!(
            url_only,
            InputModes {
                form: false,
                url: true
            }
        );
        let requests = json!({"a":{"method":"elicitation/create","params":{"mode":"form"}}});
        assert!(!url_only.covers(&requests));
        assert!(
            InputModes {
                form: true,
                url: false
            }
            .covers(&requests)
        );
        assert_eq!(InputModes::from_names(&url_only.names()), url_only);
    }
    #[test]
    fn task_view_hides_internal_queue_state() {
        let job = Job {
            id: Uuid::nil(),
            connection_id: Uuid::nil(),
            method: "tools/call".into(),
            request: json!({}),
            result: None,
            error: None,
            status: "queued".into(),
            status_message: None,
            input_requests: None,
            cancel_requested: false,
            detached: true,
            caller: Some("github:1".into()),
            input_modes: Vec::new(),
            created_at: 0,
            updated_at: 10,
            expires_at: 1800,
            claim_expires_at: Some(35),
        };
        let task = task_view(&job, 2000, true);
        assert_eq!(task["status"], "working");
        assert_eq!(task["ttlMs"], 1_800_000);
        assert_eq!(task["createdAt"], "1970-01-01T00:00:00Z");
        assert!(
            task["statusMessage"]
                .as_str()
                .is_some_and(|m| m.contains("claim"))
        );
    }
}
