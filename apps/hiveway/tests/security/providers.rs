//! Dashboard-configured provider keys against a fake Streamable HTTP upstream:
//! validation, encrypted storage, hosted tool calls, session expiry, retries
//! that never repeat executed calls, key rotation and Hive Local precedence.
use super::{Result, attack};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

#[derive(Default)]
pub(super) struct Upstream {
    key: Mutex<String>,
    sessions: Mutex<Vec<String>>,
    initialized: AtomicUsize,
    calls: AtomicUsize,
    declined: AtomicUsize,
    expire_next: AtomicBool,
}
/// The suite's single fake provider, shared by the server and the assertions.
pub(super) fn fixture() -> Arc<Upstream> {
    static UPSTREAM: std::sync::OnceLock<Arc<Upstream>> = std::sync::OnceLock::new();
    UPSTREAM
        .get_or_init(|| {
            let upstream = Upstream::default();
            if let Ok(mut key) = upstream.key.lock() {
                "good-key".clone_into(&mut key);
            }
            Arc::new(upstream)
        })
        .clone()
}
pub(super) fn router(upstream: Arc<Upstream>) -> Router {
    Router::new()
        .route("/provider/mcp", axum::routing::post(mcp))
        .with_state(upstream)
}
fn reply(id: &Value, result: &Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"result":result})
}
async fn mcp(
    State(upstream): State<Arc<Upstream>>,
    headers: HeaderMap,
    Json(message): Json<Value>,
) -> Response {
    let expected = upstream
        .key
        .lock()
        .map_or_else(|_| String::new(), |k| k.clone());
    if headers.get("authorization").and_then(|v| v.to_str().ok())
        != Some(format!("Bearer {expected}").as_str())
    {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let id = message["id"].clone();
    let method = message["method"].as_str().unwrap_or_default();
    if method == "initialize" {
        let n = upstream.initialized.fetch_add(1, Ordering::SeqCst);
        let session = format!("session-{n}");
        if let Ok(mut sessions) = upstream.sessions.lock() {
            sessions.push(session.clone());
        }
        return (
            [("mcp-session-id", session)],
            Json(reply(
                &id,
                &json!({"protocolVersion":"2025-11-25","capabilities":{"tools":{}},"serverInfo":{"name":"fake","version":"1"}}),
            )),
        )
            .into_response();
    }
    let session = headers
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    if upstream.expire_next.swap(false, Ordering::SeqCst)
        && let Ok(mut sessions) = upstream.sessions.lock()
    {
        sessions.clear();
    }
    if !upstream
        .sessions
        .lock()
        .is_ok_and(|sessions| sessions.contains(&session))
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    match method {
        "notifications/initialized" => StatusCode::ACCEPTED.into_response(),
        "tools/list" => Json(reply(
            &id,
            &json!({"tools":[
                {"name":"resolve-library-id","description":"Find a library","inputSchema":{"type":"object"}},
                {"name":"query-docs","description":"Read docs","inputSchema":{"type":"object","properties":{"q":{"type":"string"}}}},
                {"name":"bad name!","inputSchema":{"type":"object"}}
            ]}),
        ))
        .into_response(),
        "tools/call" => {
            upstream.calls.fetch_add(1, Ordering::SeqCst);
            let q = message["params"]["arguments"]["q"]
                .as_str()
                .unwrap_or_default()
                .to_owned();
            if q == "boom" {
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
            // Stream an elicitation request before the answer; Hiveway must decline it.
            let elicit = json!({"jsonrpc":"2.0","id":"elicit-1","method":"elicitation/create","params":{"message":"Which version?","requestedSchema":{"type":"object"}}});
            let answer = reply(
                &id,
                &json!({"content":[{"type":"text","text":format!("docs for {q}")}]}),
            );
            (
                [("content-type", "text/event-stream")],
                format!("event: message\ndata: {elicit}\n\nevent: message\ndata: {answer}\n\n"),
            )
                .into_response()
        }
        _ if message.get("method").is_none() => {
            if message["result"]["action"] == "decline" {
                upstream.declined.fetch_add(1, Ordering::SeqCst);
            }
            StatusCode::ACCEPTED.into_response()
        }
        _ => Json(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"no"}}))
            .into_response(),
    }
}

async fn call(app: &Router, bearer: &str, name: &str, args: Value) -> Result<Value> {
    let (status, response) = attack(
        app.clone(),
        "POST",
        "/mcp",
        json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args}}).to_string(),
        &[("authorization", bearer)],
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{response}");
    Ok(response["result"].clone())
}
async fn tool_names(app: &Router, bearer: &str) -> Result<Vec<String>> {
    let (_, listed) = attack(
        app.clone(),
        "POST",
        "/mcp",
        json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
        &[("authorization", bearer), ("x-hive-discovery", "all")],
    )
    .await?;
    Ok(listed["result"]["tools"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|t| t["name"].as_str().map(str::to_owned))
        .collect())
}
async fn providers(app: &Router, session: &[(&str, &str)]) -> Result<Value> {
    let (status, body) = attack(
        app.clone(),
        "GET",
        "/v1/account/providers",
        String::new(),
        session,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    Ok(body["providers"]
        .as_array()
        .and_then(|p| p.iter().find(|p| p["slug"] == "context7"))
        .cloned()
        .ok_or("context7 missing")?)
}
async fn save(app: &Router, session: &[(&str, &str)], key: &str) -> Result<(StatusCode, Value)> {
    attack(
        app.clone(),
        "PUT",
        "/v1/account/providers/context7",
        json!({"api_key":key}).to_string(),
        session,
    )
    .await
}
fn text(result: &Value) -> &str {
    result["content"][0]["text"].as_str().unwrap_or_default()
}

pub(super) async fn exercise(
    app: &Router,
    bearer: &str,
    session: &[(&str, &str)],
    upstream: &Upstream,
) -> Result {
    let (status, _) = attack(
        app.clone(),
        "GET",
        "/v1/account/providers",
        String::new(),
        &[("authorization", bearer)],
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "agents cannot read provider settings"
    );
    let initial = providers(app, session).await?;
    assert_eq!(initial["configured"], false);
    let (status, body) = save(app, session, "wrong-key").await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "provider_rejected_key", "{body}");
    let (status, body) = save(app, session, "bad\nkey").await?;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    let (status, saved) = save(app, session, "good-key").await?;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["tools"], 2, "invalid upstream tool names are dropped");
    assert_eq!(saved["hint"], "••••-key");
    let listed = providers(app, session).await?;
    assert_eq!(listed["configured"], true);
    assert_eq!(listed["key"]["served_by"], "hive");
    assert!(
        !listed.to_string().contains("good-key"),
        "keys are never returned"
    );
    let names = tool_names(app, bearer).await?;
    assert!(
        names.iter().any(|n| n == "mcp_context7__query-docs"),
        "{names:?}"
    );
    hosted_calls(app, bearer, session, upstream).await?;
    local_precedence(app, bearer, upstream).await?;
    let (status, removed) = attack(
        app.clone(),
        "DELETE",
        "/v1/account/providers/context7",
        String::new(),
        session,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(removed["removed"], true);
    assert_eq!(providers(app, session).await?["configured"], false);
    let names = tool_names(app, bearer).await?;
    assert!(
        !names.iter().any(|n| n.starts_with("mcp_context7__")),
        "{names:?}"
    );
    Ok(())
}

/// Tool calls through the hosted connection: caching, safe retries and key rotation.
async fn hosted_calls(
    app: &Router,
    bearer: &str,
    session: &[(&str, &str)],
    upstream: &Upstream,
) -> Result {
    let first = call(app, bearer, "mcp_context7__query-docs", json!({"q":"axum"})).await?;
    assert_eq!(first["isError"], false, "{first}");
    assert_eq!(text(&first), "docs for axum");
    assert_eq!(
        upstream.declined.load(Ordering::SeqCst),
        1,
        "elicitation is declined"
    );
    let calls = upstream.calls.load(Ordering::SeqCst);
    let cached = call(app, bearer, "mcp_context7__query-docs", json!({"q":"axum"})).await?;
    assert_eq!(text(&cached), "docs for axum");
    assert_eq!(
        upstream.calls.load(Ordering::SeqCst),
        calls,
        "read-only results are cached"
    );

    // An expired session is rejected before execution, so one transparent retry is safe.
    let sessions = upstream.initialized.load(Ordering::SeqCst);
    upstream.expire_next.store(true, Ordering::SeqCst);
    let renewed = call(
        app,
        bearer,
        "mcp_context7__query-docs",
        json!({"q":"tokio"}),
    )
    .await?;
    assert_eq!(text(&renewed), "docs for tokio", "{renewed}");
    assert_eq!(upstream.initialized.load(Ordering::SeqCst), sessions + 1);

    // A server error after sending may have executed: report it, never repeat it.
    let calls = upstream.calls.load(Ordering::SeqCst);
    let failed = call(app, bearer, "mcp_context7__query-docs", json!({"q":"boom"})).await?;
    assert_eq!(failed["isError"], true, "{failed}");
    assert!(text(&failed).contains("outcome is unknown"), "{failed}");
    assert_eq!(upstream.calls.load(Ordering::SeqCst), calls + 1);

    // The provider revokes the key: the call fails clearly and the dashboard shows it.
    if let Ok(mut key) = upstream.key.lock() {
        "new-key".clone_into(&mut key);
    }
    let rejected = call(
        app,
        bearer,
        "mcp_context7__query-docs",
        json!({"q":"serde"}),
    )
    .await?;
    assert_eq!(rejected["isError"], true);
    assert!(text(&rejected).contains("rejected the saved API key"));
    assert_eq!(
        providers(app, session).await?["key"]["last_error"],
        "rejected_by_provider"
    );
    let (status, _) = save(app, session, "new-key").await?;
    assert_eq!(status, StatusCode::OK);
    assert!(providers(app, session).await?["key"]["last_error"].is_null());
    let (status, refreshed) = attack(
        app.clone(),
        "POST",
        "/v1/account/providers/context7/refresh",
        "{}".into(),
        session,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{refreshed}");
    let rotated = call(
        app,
        bearer,
        "mcp_context7__query-docs",
        json!({"q":"serde"}),
    )
    .await?;
    assert_eq!(text(&rotated), "docs for serde", "{rotated}");
    Ok(())
}

/// An online Hive Local connection with the provider's name serves its calls.
async fn local_precedence(app: &Router, bearer: &str, upstream: &Upstream) -> Result {
    let registered = call(
        app,
        bearer,
        "register_mcp_connection",
        json!({"name":"context7","features":["long_poll"],"tools":[{"name":"query-docs","inputSchema":{"type":"object"}}]}),
    )
    .await?;
    assert_eq!(registered["isError"], false, "{registered}");
    let connection = registered["structuredContent"]["id"]
        .as_str()
        .ok_or("missing connection")?
        .to_owned();
    let calls = upstream.calls.load(Ordering::SeqCst);
    let worker = async {
        let polled = call(
            app,
            bearer,
            "poll_mcp",
            json!({"connection_id":connection,"wait_seconds":8}),
        )
        .await?;
        let job = polled["structuredContent"]["job"].clone();
        assert!(!job.is_null(), "the local connection receives the job");
        let done = call(
            app,
            bearer,
            "complete_mcp",
            json!({"connection_id":connection,"job_id":job["id"],"lease":job["lease"],"result":{"content":[{"type":"text","text":"from hive local"}]}}),
        )
        .await?;
        assert_eq!(done["isError"], false, "{done}");
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let caller = call(
        app,
        bearer,
        "mcp_context7__query-docs",
        json!({"q":"local"}),
    );
    let (served, answered) = tokio::join!(worker, caller);
    served?;
    let answered = answered?;
    assert_eq!(text(&answered), "from hive local", "{answered}");
    assert_eq!(upstream.calls.load(Ordering::SeqCst), calls);
    let removed = call(
        app,
        bearer,
        "disconnect_mcp",
        json!({"connection_id":connection}),
    )
    .await?;
    assert_eq!(removed["isError"], false);
    Ok(())
}
