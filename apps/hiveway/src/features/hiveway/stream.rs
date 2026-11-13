//! Server-to-client notifications.
//!
//! 2026-07-28 clients open `subscriptions/listen` (a long-lived POST response
//! stream) and opt into list changes and task status. Earlier clients use the
//! HTTP GET stream. Both are fed by realtime wakeups for the caller's
//! connections and tasks, debounced, bounded in lifetime and count, and close
//! gracefully with the listen result so clients can reconnect.
use crate::{
    features::{
        common::{error::ApiError, model::Credential},
        connections,
        realtime::{SubscriptionSlot, job_key, owner_key},
    },
    state::AppState,
};
use axum::response::{
    IntoResponse, Response,
    sse::{Event, KeepAlive, Sse},
};
use futures_util::stream::{self, Stream};
use serde_json::{Value, json};
use std::{collections::VecDeque, convert::Infallible, sync::Arc, time::Duration};
use tokio::sync::Notify;

#[derive(Clone, Copy, Default)]
struct Filter {
    tools: bool,
    prompts: bool,
    resources: bool,
}
struct Listener {
    state: AppState,
    credential: Credential,
    /// JSON-RPC id of the listen request; `None` for the legacy GET stream.
    id: Option<Value>,
    filter: Filter,
    tasks: Vec<String>,
    lists: Arc<Notify>,
    task_changes: Arc<Notify>,
    deadline: tokio::time::Instant,
    last: std::collections::HashMap<String, String>,
    queue: VecDeque<Value>,
    primed: bool,
    finished: bool,
    _slot: SubscriptionSlot,
}
impl Listener {
    fn tag(&self, mut message: Value) -> Value {
        if let Some(id) = &self.id {
            message["params"]["_meta"]["io.modelcontextprotocol/subscriptionId"] = id.clone();
        }
        message
    }
    fn list_changes(&mut self) {
        for (enabled, method) in [
            (self.filter.tools, "notifications/tools/list_changed"),
            (self.filter.prompts, "notifications/prompts/list_changed"),
            (
                self.filter.resources,
                "notifications/resources/list_changed",
            ),
        ] {
            if enabled {
                let message = self.tag(json!({"jsonrpc":"2.0","method":method,"params":{}}));
                self.queue.push_back(message);
            }
        }
    }
    async fn task_changes(&mut self) {
        for task in self.tasks.clone() {
            let Ok((view, _)) = connections::task(&self.state, &self.credential, &task).await
            else {
                continue;
            };
            let version = format!(
                "{}:{}:{}",
                view["status"], view["lastUpdatedAt"], view["statusMessage"]
            );
            if self.last.get(&task) == Some(&version) {
                continue;
            }
            self.last.insert(task, version);
            let message =
                self.tag(json!({"jsonrpc":"2.0","method":"notifications/tasks","params":view}));
            self.queue.push_back(message);
        }
    }
    async fn next_message(&mut self) -> Option<Value> {
        loop {
            if let Some(message) = self.queue.pop_front() {
                return Some(message);
            }
            if self.finished {
                return None;
            }
            if !self.primed {
                // Report current task states right after the acknowledgement.
                self.primed = true;
                self.task_changes().await;
                continue;
            }
            let lists = self.lists.clone();
            let tasks = self.task_changes.clone();
            let list_wait = lists.notified();
            let task_wait = tasks.notified();
            tokio::pin!(list_wait, task_wait);
            tokio::select! {
                () = &mut list_wait => {
                    // Coalesce bursts such as several connectors re-registering at once.
                    tokio::time::sleep(Duration::from_millis(250)).await;
                    self.list_changes();
                }
                () = &mut task_wait => self.task_changes().await,
                () = tokio::time::sleep_until(self.deadline) => {
                    self.finished = true;
                    if let Some(id) = &self.id {
                        self.queue.push_back(json!({"jsonrpc":"2.0","id":id,"result":{"resultType":"complete","_meta":{"io.modelcontextprotocol/subscriptionId":id}}}));
                    }
                }
            }
        }
    }
}
fn events(listener: Listener, first: Vec<Value>) -> impl Stream<Item = Result<Event, Infallible>> {
    let mut listener = listener;
    listener.queue.extend(first);
    stream::unfold(listener, |mut listener| async move {
        let message = listener.next_message().await?;
        let event = Event::default().event("message").data(message.to_string());
        Some((Ok(event), listener))
    })
}

/// Open a notification stream for the caller. `id` is the listen request id
/// (modern) or `None` for the legacy GET stream, which receives every list change.
/// # Errors
/// Rejects callers over their subscription allowance.
pub fn open(
    state: &AppState,
    credential: Credential,
    id: Option<Value>,
    params: &Value,
) -> Result<Response, ApiError> {
    let mcp = &state.settings.runtime.mcp;
    let slot = state
        .realtime
        .subscription(credential.id, mcp.subscriptions_per_credential)
        .ok_or_else(|| {
            ApiError::new(
                axum::http::StatusCode::TOO_MANY_REQUESTS,
                "subscription_limit",
            )
        })?;
    let requested = &params["notifications"];
    let filter = if id.is_some() {
        Filter {
            tools: requested["toolsListChanged"] == true,
            prompts: requested["promptsListChanged"] == true,
            resources: requested["resourcesListChanged"] == true,
        }
    } else {
        Filter {
            tools: true,
            prompts: true,
            resources: true,
        }
    };
    let tasks: Vec<String> = requested["taskIds"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|t| {
            t.as_str()
                .filter(|t| t.parse::<uuid::Uuid>().is_ok())
                .map(str::to_owned)
        })
        .take(32)
        .collect();
    let task_keys: Vec<String> = tasks
        .iter()
        .filter_map(|t| t.parse().ok())
        .map(job_key)
        .collect();
    let mut first = Vec::new();
    if let Some(id) = &id {
        let mut accepted = json!({
            "toolsListChanged": filter.tools,
            "promptsListChanged": filter.prompts,
            "resourcesListChanged": filter.resources,
        });
        if !tasks.is_empty() {
            accepted["taskIds"] = json!(tasks);
        }
        first.push(json!({"jsonrpc":"2.0","method":"notifications/subscriptions/acknowledged","params":{"notifications":accepted,"_meta":{"io.modelcontextprotocol/subscriptionId":id}}}));
    }
    let listener = Listener {
        lists: state.realtime.watch(&[owner_key(&credential.wallet)]),
        task_changes: state.realtime.watch(&task_keys),
        state: state.clone(),
        credential,
        id,
        filter,
        tasks,
        deadline: tokio::time::Instant::now() + Duration::from_secs(mcp.subscription_seconds),
        last: std::collections::HashMap::new(),
        queue: VecDeque::new(),
        primed: false,
        finished: false,
        _slot: slot,
    };
    Ok(Sse::new(events(listener, first))
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(mcp.keepalive_seconds)))
        .into_response())
}
