//! End-to-end checks for the agent platform: connector long-polls and caching,
//! Multi Round-Trip input, MCP tasks, bridged resources and prompts, change
//! subscriptions, repository memory, the code index and MCP OAuth.
use super::{Result, attack};
use axum::{
    Router,
    body::Body,
    extract::ConnectInfo,
    http::{HeaderValue, Request, StatusCode},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use futures_util::StreamExt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{net::SocketAddr, time::Duration};
use tower::ServiceExt;

const MODERN: &str = "2026-07-28";

async fn rpc(app: &Router, bearer: &str, name: &str, args: Value) -> Result<Value> {
    let (_, response) = attack(
        app.clone(),
        "POST",
        "/mcp",
        json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args}}).to_string(),
        &[("authorization", bearer)],
    )
    .await?;
    Ok(response["result"].clone())
}
fn modern_params(params: Value, capabilities: &Value) -> Value {
    let mut params = params;
    params["_meta"] = json!({
        "io.modelcontextprotocol/protocolVersion": MODERN,
        "io.modelcontextprotocol/clientInfo": {"name":"platform-test","version":"1"},
        "io.modelcontextprotocol/clientCapabilities": capabilities,
    });
    params
}
/// A 2026-07-28 request with matching routing headers.
async fn modern(
    app: &Router,
    bearer: &str,
    method: &str,
    params: Value,
    capabilities: &Value,
) -> Result<Value> {
    let name = params["name"].as_str().unwrap_or_default().to_owned();
    let mut headers = vec![
        ("authorization", bearer),
        ("mcp-protocol-version", MODERN),
        ("mcp-method", method),
    ];
    if !name.is_empty() {
        headers.push(("mcp-name", name.as_str()));
    }
    let (status, response) = attack(
        app.clone(),
        "POST",
        "/mcp",
        json!({"jsonrpc":"2.0","id":uuid::Uuid::new_v4().to_string(),"method":method,"params":modern_params(params, capabilities)}).to_string(),
        &headers,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{response}");
    Ok(response)
}
async fn claim(app: &Router, bearer: &str, connection: &str) -> Result<Value> {
    let polled = rpc(
        app,
        bearer,
        "poll_mcp",
        json!({"connection_id":connection,"wait_seconds":8}),
    )
    .await?;
    let job = polled["structuredContent"]["job"].clone();
    if job.is_null() {
        return Err("long-poll returned without a job".into());
    }
    Ok(job)
}
async fn complete(
    app: &Router,
    bearer: &str,
    connection: &str,
    job: &Value,
    result: Value,
) -> Result {
    let done = rpc(
        app,
        bearer,
        "complete_mcp",
        json!({"connection_id":connection,"job_id":job["id"],"lease":job["lease"],"result":result}),
    )
    .await?;
    assert_eq!(done["isError"], false, "{done}");
    Ok(())
}
fn text(value: &str) -> Value {
    json!({"content":[{"type":"text","text":value}]})
}

pub(super) async fn exercise(
    app: Router,
    bearer: &str,
    session: &[(&str, &str)],
    state: &hiveway::state::AppState,
) -> Result {
    let connection = connector_surface(&app, bearer).await?;
    long_poll_and_cache(&app, bearer, &connection).await?;
    multi_round_trip(&app, bearer, &connection).await?;
    tasks(&app, bearer, &connection).await?;
    bridged(&app, bearer, &connection).await?;
    subscriptions(&app, bearer).await?;
    memory(&app, bearer).await?;
    code_index(&app, bearer).await?;
    oauth(&app, bearer, session, state).await?;
    let removed = rpc(
        &app,
        bearer,
        "disconnect_mcp",
        json!({"connection_id":connection}),
    )
    .await?;
    assert_eq!(removed["isError"], false);
    Ok(())
}

fn snapshot(lookup_schema: &Value) -> Value {
    json!({
        "name": "platform",
        "features": ["bridge","input","progress","cancel","long_poll"],
        "tools": [
            {"name":"lookup","inputSchema":lookup_schema,"annotations":{"readOnlyHint":true,"openWorldHint":false,"title":"Look up"}},
            {"name":"deploy","inputSchema":{"type":"object"}}
        ],
        "resources": [{"uri":"file:///README.md","name":"readme"}],
        "resource_templates": [{"uriTemplate":"file:///{path}","name":"files"}],
        "prompts": [{"name":"review","arguments":[{"name":"focus"}]}],
    })
}
async fn connector_surface(app: &Router, bearer: &str) -> Result<String> {
    let registered = rpc(
        app,
        bearer,
        "register_mcp_connection",
        snapshot(&json!({"type":"object"})),
    )
    .await?;
    assert_eq!(registered["isError"], false, "{registered}");
    let connection = registered["structuredContent"]["id"]
        .as_str()
        .ok_or("missing connection")?
        .to_owned();
    assert!(
        registered["structuredContent"]["limits"]["result_bytes"]
            .as_u64()
            .is_some()
    );
    let (_, listed) = attack(
        app.clone(),
        "POST",
        "/mcp",
        json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
        &[("authorization", bearer), ("x-hive-discovery", "all")],
    )
    .await?;
    let tools = listed["result"]["tools"]
        .as_array()
        .ok_or("missing tools")?;
    let find = |name: &str| tools.iter().find(|t| t["name"] == name).cloned();
    let lookup = find("mcp_platform__lookup").ok_or("lookup not listed")?;
    assert_eq!(
        lookup["annotations"]["readOnlyHint"], true,
        "upstream hints pass through"
    );
    assert_eq!(lookup["annotations"]["destructiveHint"], false);
    assert_eq!(lookup["annotations"]["openWorldHint"], false);
    let deploy = find("mcp_platform__deploy").ok_or("deploy not listed")?;
    assert_eq!(
        deploy["annotations"]["destructiveHint"], true,
        "silent upstreams stay conservative"
    );
    for native in [
        "get_task",
        "remember",
        "recall",
        "code_search",
        "index_repository",
    ] {
        assert!(find(native).is_some(), "{native} missing from tools/list");
    }
    assert!(listed["result"]["ttlMs"].as_u64().is_some());
    assert_eq!(listed["result"]["cacheScope"], "private");
    Ok(connection)
}

async fn long_poll_and_cache(app: &Router, bearer: &str, connection: &str) -> Result {
    let started = std::time::Instant::now();
    let worker = async {
        let job = claim(app, bearer, connection).await?;
        assert_eq!(job["request"]["name"], "lookup");
        assert_eq!(job["method"], "tools/call");
        complete(app, bearer, connection, &job, text("found it")).await
    };
    let call = async {
        // Let the worker park in its long-poll first.
        tokio::time::sleep(Duration::from_millis(200)).await;
        rpc(app, bearer, "mcp_platform__lookup", json!({"q":"x","n":1})).await
    };
    let (finished, called) = tokio::join!(worker, call);
    finished?;
    let called = called?;
    assert_eq!(called["content"][0]["text"], "found it", "{called}");
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "long-poll woke on enqueue: {:?}",
        started.elapsed()
    );
    // A read-only result is served from cache without a connector round trip.
    let cached = rpc(app, bearer, "mcp_platform__lookup", json!({"n":1,"q":"x"})).await?;
    assert_eq!(cached["content"][0]["text"], "found it");
    assert_eq!(cached["_meta"]["dev.hive/cache"]["hit"], true, "{cached}");
    // A changed snapshot purges the cache; with no worker the call becomes a job handle.
    let changed = rpc(
        app,
        bearer,
        "register_mcp_connection",
        snapshot(&json!({"type":"object","properties":{"q":{"type":"string"}}})),
    )
    .await?;
    assert_eq!(changed["structuredContent"]["changed"], true);
    let pending = rpc(app, bearer, "mcp_platform__lookup", json!({"q":"x","n":1})).await?;
    assert_eq!(pending["isError"], false, "{pending}");
    let task = pending["structuredContent"]["task"]["taskId"]
        .as_str()
        .ok_or("missing task handle")?
        .to_owned();
    let worker = async {
        let job = claim(app, bearer, connection).await?;
        complete(app, bearer, connection, &job, text("fresh")).await
    };
    let follow = rpc(
        app,
        bearer,
        "get_task",
        json!({"task_id":task,"wait_seconds":8}),
    );
    let (finished, followed) = tokio::join!(worker, follow);
    finished?;
    let followed = followed?;
    if followed["content"][0]["text"] != "fresh" {
        // The first wait may return on the claim; the next one sees completion.
        let again = rpc(
            app,
            bearer,
            "get_task",
            json!({"task_id":task,"wait_seconds":8}),
        )
        .await?;
        assert_eq!(again["content"][0]["text"], "fresh", "{again}");
    }
    Ok(())
}

async fn multi_round_trip(app: &Router, bearer: &str, connection: &str) -> Result {
    let capabilities = json!({"elicitation":{"form":{}}});
    let call = json!({"name":"mcp_platform__deploy","arguments":{"env":"prod"}});
    let worker = async {
        let job = claim(app, bearer, connection).await?;
        let asked = rpc(app, bearer, "elicit_mcp", json!({"connection_id":connection,"job_id":job["id"],"lease":job["lease"],"requests":{"confirm":{"method":"elicitation/create","params":{"mode":"form","message":"Deploy to prod?","requestedSchema":{"type":"object","properties":{"ok":{"type":"boolean"}}}}}}})).await?;
        assert_eq!(asked["structuredContent"]["forwarded"], true, "{asked}");
        Ok::<Value, Box<dyn std::error::Error>>(job)
    };
    let (job, first) = tokio::join!(
        worker,
        modern(app, bearer, "tools/call", call.clone(), &capabilities)
    );
    let job = job?;
    let first = first?;
    assert_eq!(first["result"]["resultType"], "input_required", "{first}");
    assert_eq!(
        first["result"]["inputRequests"]["confirm"]["params"]["message"],
        "Deploy to prod?"
    );
    let request_state = first["result"]["requestState"].clone();
    let mut retry = call.clone();
    retry["requestState"] = request_state;
    retry["inputResponses"] = json!({"confirm":{"action":"accept","content":{"ok":true}}});
    let worker = async {
        let answer = rpc(app, bearer, "await_input_mcp", json!({"connection_id":connection,"job_id":job["id"],"lease":job["lease"],"wait_seconds":8})).await?;
        assert_eq!(
            answer["structuredContent"]["responses"]["confirm"]["action"], "accept",
            "{answer}"
        );
        let progress = rpc(app, bearer, "progress_mcp", json!({"connection_id":connection,"job_id":job["id"],"lease":job["lease"],"message":"rolling out","progress":1,"total":2})).await?;
        assert_eq!(progress["structuredContent"]["cancel_requested"], false);
        complete(app, bearer, connection, &job, text("deployed")).await
    };
    let (finished, second) = tokio::join!(
        worker,
        modern(app, bearer, "tools/call", retry, &capabilities)
    );
    finished?;
    let second = second?;
    assert_eq!(second["result"]["resultType"], "complete", "{second}");
    assert_eq!(second["result"]["content"][0]["text"], "deployed");
    // A legacy client cannot show forms: the connector receives a decline instead.
    let worker = async {
        let job = claim(app, bearer, connection).await?;
        let asked = rpc(app, bearer, "elicit_mcp", json!({"connection_id":connection,"job_id":job["id"],"lease":job["lease"],"requests":{"confirm":{"method":"elicitation/create","params":{"mode":"form","message":"Sure?","requestedSchema":{"type":"object"}}}}})).await?;
        assert_eq!(asked["structuredContent"]["forwarded"], false);
        let answer = rpc(app, bearer, "await_input_mcp", json!({"connection_id":connection,"job_id":job["id"],"lease":job["lease"],"wait_seconds":2})).await?;
        assert_eq!(
            answer["structuredContent"]["responses"]["confirm"]["action"],
            "decline"
        );
        complete(app, bearer, connection, &job, text("skipped")).await
    };
    let (finished, legacy) = tokio::join!(
        worker,
        rpc(app, bearer, "mcp_platform__deploy", json!({"env":"dev"}))
    );
    finished?;
    assert_eq!(legacy?["content"][0]["text"], "skipped");
    Ok(())
}

async fn tasks(app: &Router, bearer: &str, connection: &str) -> Result {
    let capabilities =
        json!({"elicitation":{"form":{}},"extensions":{"io.modelcontextprotocol/tasks":{}}});
    let started = modern(
        app,
        bearer,
        "tools/call",
        json!({"name":"mcp_platform__deploy","arguments":{"env":"stage"}}),
        &capabilities,
    )
    .await?;
    assert_eq!(started["result"]["resultType"], "task", "{started}");
    let task = started["result"]["taskId"]
        .as_str()
        .ok_or("missing taskId")?
        .to_owned();
    assert_eq!(started["result"]["status"], "working");
    let get = |task: String| {
        modern(
            app,
            bearer,
            "tasks/get",
            json!({"taskId":task}),
            &capabilities,
        )
    };
    let job = claim(app, bearer, connection).await?;
    rpc(app, bearer, "elicit_mcp", json!({"connection_id":connection,"job_id":job["id"],"lease":job["lease"],"requests":{"approve":{"method":"elicitation/create","params":{"mode":"form","message":"Approve stage?","requestedSchema":{"type":"object"}}}}})).await?;
    let waiting = get(task.clone()).await?;
    assert_eq!(waiting["result"]["status"], "input_required", "{waiting}");
    assert!(waiting["result"]["inputRequests"]["approve"].is_object());
    let updated = modern(
        app,
        bearer,
        "tasks/update",
        json!({"taskId":task,"inputResponses":{"approve":{"action":"accept","content":{}}}}),
        &capabilities,
    )
    .await?;
    assert_eq!(updated["result"]["resultType"], "complete");
    let answer = rpc(app, bearer, "await_input_mcp", json!({"connection_id":connection,"job_id":job["id"],"lease":job["lease"],"wait_seconds":5})).await?;
    assert_eq!(
        answer["structuredContent"]["responses"]["approve"]["action"],
        "accept"
    );
    complete(app, bearer, connection, &job, text("staged")).await?;
    let done = get(task.clone()).await?;
    assert_eq!(done["result"]["status"], "completed", "{done}");
    assert_eq!(done["result"]["result"]["content"][0]["text"], "staged");
    // Unclaimed tasks cancel immediately.
    let second = modern(
        app,
        bearer,
        "tools/call",
        json!({"name":"mcp_platform__deploy","arguments":{"env":"cancel"}}),
        &capabilities,
    )
    .await?;
    let second = second["result"]["taskId"]
        .as_str()
        .ok_or("missing second task")?
        .to_owned();
    modern(
        app,
        bearer,
        "tasks/cancel",
        json!({"taskId":second}),
        &capabilities,
    )
    .await?;
    let cancelled = get(second).await?;
    assert_eq!(cancelled["result"]["status"], "cancelled", "{cancelled}");
    let foreign = get(uuid::Uuid::new_v4().to_string()).await?;
    assert_eq!(
        foreign["error"]["code"], -32602,
        "unknown tasks are not disclosed"
    );
    Ok(())
}

async fn bridged(app: &Router, bearer: &str, connection: &str) -> Result {
    let capabilities = json!({});
    let listed = modern(app, bearer, "resources/list", json!({}), &capabilities).await?;
    let uri = "hive-mcp://platform/file:///README.md";
    assert!(
        listed["result"]["resources"]
            .as_array()
            .is_some_and(|r| r.iter().any(|x| x["uri"] == uri)),
        "{listed}"
    );
    let templates = modern(
        app,
        bearer,
        "resources/templates/list",
        json!({}),
        &capabilities,
    )
    .await?;
    assert_eq!(
        templates["result"]["resourceTemplates"][0]["uriTemplate"],
        "hive-mcp://platform/file:///{path}"
    );
    let worker = async {
        let job = claim(app, bearer, connection).await?;
        assert_eq!(job["method"], "resources/read");
        assert_eq!(job["request"]["params"]["uri"], "file:///src/lib.rs");
        complete(
            app,
            bearer,
            connection,
            &job,
            json!({"contents":[{"uri":"file:///src/lib.rs","text":"pub fn x() {}"}]}),
        )
        .await
    };
    let (finished, read) = tokio::join!(
        worker,
        modern(
            app,
            bearer,
            "resources/read",
            json!({"uri":"hive-mcp://platform/file:///src/lib.rs"}),
            &capabilities
        )
    );
    finished?;
    let read = read?;
    assert_eq!(
        read["result"]["contents"][0]["uri"], "hive-mcp://platform/file:///src/lib.rs",
        "{read}"
    );
    let refused = modern(
        app,
        bearer,
        "resources/read",
        json!({"uri":"hive-mcp://platform/https://evil.example/"}),
        &capabilities,
    )
    .await?;
    assert_eq!(
        refused["error"]["code"], -32602,
        "unlisted URIs never reach the connector"
    );
    let prompts = modern(app, bearer, "prompts/list", json!({}), &capabilities).await?;
    assert_eq!(prompts["result"]["prompts"][0]["name"], "platform__review");
    let worker = async {
        let job = claim(app, bearer, connection).await?;
        assert_eq!(
            job["request"]["params"],
            json!({"name":"review","arguments":{"focus":"security"}})
        );
        complete(app, bearer, connection, &job, json!({"messages":[{"role":"user","content":{"type":"text","text":"Review for security"}}]})).await
    };
    let (finished, prompt) = tokio::join!(
        worker,
        modern(
            app,
            bearer,
            "prompts/get",
            json!({"name":"platform__review","arguments":{"focus":"security"}}),
            &capabilities
        )
    );
    finished?;
    assert_eq!(
        prompt?["result"]["messages"][0]["content"]["text"],
        "Review for security"
    );
    Ok(())
}

async fn next_event(
    body: &mut (
             impl futures_util::Stream<Item = std::result::Result<axum::body::Bytes, axum::Error>>
             + Unpin
         ),
    buffer: &mut String,
) -> Result<Value> {
    loop {
        if let Some(end) = buffer.find("\n\n") {
            let event: String = buffer.drain(..end + 2).collect();
            if let Some(data) = event.lines().find_map(|l| l.strip_prefix("data: ")) {
                return Ok(serde_json::from_str(data)?);
            }
            continue;
        }
        let chunk = tokio::time::timeout(Duration::from_secs(5), body.next())
            .await?
            .ok_or("stream ended")??;
        buffer.push_str(&String::from_utf8_lossy(&chunk));
    }
}
async fn subscriptions(app: &Router, bearer: &str) -> Result {
    let mut request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .header("host", "127.0.0.1:3001")
        .header("authorization", bearer)
        .header("mcp-protocol-version", MODERN)
        .header("mcp-method", "subscriptions/listen")
        .body(Body::from(
            json!({"jsonrpc":"2.0","id":"listen-1","method":"subscriptions/listen","params":modern_params(json!({"notifications":{"toolsListChanged":true}}), &json!({}))}).to_string(),
        ))?;
    request
        .extensions_mut()
        .insert(ConnectInfo("192.0.2.9:4000".parse::<SocketAddr>()?));
    let response = app.clone().oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let mut body = response.into_body().into_data_stream();
    let mut buffer = String::new();
    let ack = next_event(&mut body, &mut buffer).await?;
    assert_eq!(
        ack["method"], "notifications/subscriptions/acknowledged",
        "{ack}"
    );
    assert_eq!(ack["params"]["notifications"]["toolsListChanged"], true);
    assert_eq!(
        ack["params"]["_meta"]["io.modelcontextprotocol/subscriptionId"],
        "listen-1"
    );
    let second = rpc(
        app,
        bearer,
        "register_mcp_connection",
        json!({"name":"second","tools":[{"name":"ping","inputSchema":{"type":"object"}}]}),
    )
    .await?;
    let changed = next_event(&mut body, &mut buffer).await?;
    assert_eq!(
        changed["method"], "notifications/tools/list_changed",
        "{changed}"
    );
    assert_eq!(
        changed["params"]["_meta"]["io.modelcontextprotocol/subscriptionId"],
        "listen-1"
    );
    drop(body);
    let id = second["structuredContent"]["id"].clone();
    rpc(app, bearer, "disconnect_mcp", json!({"connection_id":id})).await?;
    Ok(())
}

async fn memory(app: &Router, bearer: &str) -> Result {
    let repo = "git@github.com:Acme/Demo.git";
    let first = rpc(app, bearer, "remember", json!({"repo":repo,"kind":"decision","title":"Package manager","body":"Use npm.","tags":["tooling"]})).await?;
    assert_eq!(first["structuredContent"]["created"], true, "{first}");
    assert_eq!(
        first["structuredContent"]["memory"]["repo"],
        "github.com/acme/demo"
    );
    let restated = rpc(app, bearer, "remember", json!({"repo":"https://github.com/acme/demo","kind":"decision","title":"package manager","body":"Use bun, not npm.","tags":["tooling"]})).await?;
    assert_eq!(
        restated["structuredContent"]["created"], false,
        "same title updates in place"
    );
    let id = restated["structuredContent"]["memory"]["id"].clone();
    rpc(app, bearer, "remember", json!({"repo":"global","kind":"preference","title":"Commit style","body":"Write imperative commit subjects."})).await?;
    let recalled = rpc(app, bearer, "recall", json!({"repo":repo})).await?;
    let memories = recalled["structuredContent"]["memories"]
        .as_array()
        .ok_or("missing memories")?;
    assert_eq!(memories.len(), 2, "{recalled}");
    let searched = rpc(
        app,
        bearer,
        "recall",
        json!({"repo":repo,"query":"bun","include_global":false}),
    )
    .await?;
    assert_eq!(
        searched["structuredContent"]["memories"][0]["body"],
        "Use bun, not npm."
    );
    let replaced = rpc(app, bearer, "remember", json!({"repo":repo,"kind":"decision","title":"Runtime","body":"Bun is the runtime and package manager.","supersedes":id})).await?;
    assert_eq!(replaced["isError"], false, "{replaced}");
    let current = rpc(
        app,
        bearer,
        "recall",
        json!({"repo":repo,"kinds":["decision"]}),
    )
    .await?;
    let titles: Vec<&str> = current["structuredContent"]["memories"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| m["title"].as_str())
        .collect();
    assert_eq!(titles, vec!["Runtime"], "superseded decisions are hidden");
    let missing = rpc(app, bearer, "forget", json!({"id":uuid::Uuid::new_v4()})).await?;
    assert_eq!(missing["isError"], true);
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn code_index(app: &Router, bearer: &str) -> Result {
    use hiveway::features::codeintel::blob_id;
    let files = [
        (
            "src/lib.rs",
            "pub fn parse_config(path: &str) -> Config {\n    load(path)\n}\npub struct Config;\n",
        ),
        (
            "src/main.rs",
            "fn main() {\n    let c = parse_config(\"x\");\n}\n",
        ),
        (
            "package.json",
            r#"{"name":"demo","scripts":{"test":"vitest"},"dependencies":{"react":"19"}}"#,
        ),
    ];
    let manifest: Vec<Value> = files.iter().map(|(path, content)| json!({"path":path,"sha":blob_id(content.as_bytes()),"bytes":content.len()})).collect();
    let repo = "https://github.com/Acme/Demo.git";
    let commit = "a".repeat(40);
    let indexed = rpc(
        app,
        bearer,
        "index_repository",
        json!({"repo":repo,"commit":commit,"ref":"main","files":manifest}),
    )
    .await?;
    assert_eq!(
        indexed["structuredContent"]["missing"]
            .as_array()
            .map(Vec::len),
        Some(3),
        "{indexed}"
    );
    let forged = rpc(
        app,
        bearer,
        "upload_code_blobs",
        json!({"blobs":[{"sha":blob_id(b"real"),"content":"forged"}]}),
    )
    .await?;
    assert_eq!(forged["isError"], true, "content must match its blob id");
    let blobs: Vec<Value> = files.iter().map(|(path, content)| json!({"sha":blob_id(content.as_bytes()),"path":path,"content":content})).collect();
    let uploaded = rpc(app, bearer, "upload_code_blobs", json!({"blobs":blobs})).await?;
    assert_eq!(uploaded["structuredContent"]["stored"], 3, "{uploaded}");
    let finalized = rpc(
        app,
        bearer,
        "index_repository",
        json!({"repo":repo,"commit":commit,"finalize":true}),
    )
    .await?;
    assert_eq!(finalized["structuredContent"]["status"], "ready");
    let stack = finalized["structuredContent"]["summary"]["stack"].to_string();
    assert!(stack.contains("rust") && stack.contains("react"), "{stack}");
    let short = "github.com/acme/demo";
    let found = rpc(
        app,
        bearer,
        "code_search",
        json!({"repo":short,"query":"parseConfig"}),
    )
    .await?;
    let paths: Vec<&str> = found["structuredContent"]["results"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| r["path"].as_str())
        .collect();
    assert!(
        paths.contains(&"src/lib.rs") && paths.contains(&"src/main.rs"),
        "{found}"
    );
    let literal = rpc(
        app,
        bearer,
        "code_search",
        json!({"repo":short,"query":"Config;","mode":"literal"}),
    )
    .await?;
    assert_eq!(
        literal["structuredContent"]["results"][0]["path"],
        "src/lib.rs"
    );
    assert_eq!(
        literal["structuredContent"]["results"][0]["snippets"][0]["line"],
        4
    );
    let symbol = rpc(
        app,
        bearer,
        "find_symbol",
        json!({"repo":short,"name":"parse_config"}),
    )
    .await?;
    assert_eq!(
        symbol["structuredContent"]["symbols"][0]["path"], "src/lib.rs",
        "{symbol}"
    );
    assert_eq!(
        symbol["structuredContent"]["symbols"][0]["kind"],
        "function"
    );
    let prefix = rpc(
        app,
        bearer,
        "find_symbol",
        json!({"repo":short,"name":"Conf"}),
    )
    .await?;
    assert_eq!(prefix["structuredContent"]["symbols"][0]["name"], "Config");
    let references = rpc(
        app,
        bearer,
        "find_references",
        json!({"repo":short,"symbol":"parse_config"}),
    )
    .await?;
    assert_eq!(
        references["structuredContent"]["references"]
            .as_array()
            .map(Vec::len),
        Some(2),
        "{references}"
    );
    let read = rpc(
        app,
        bearer,
        "read_code",
        json!({"repo":short,"path":"src/main.rs","start_line":2,"end_line":2}),
    )
    .await?;
    assert!(
        read["structuredContent"]["text"]
            .as_str()
            .is_some_and(|t| t.starts_with("2 | ")),
        "{read}"
    );
    // The next commit only uploads what changed.
    let next = "b".repeat(40);
    let mut changed = manifest.clone();
    let main = "fn main() {}\n";
    changed[1] = json!({"path":"src/main.rs","sha":blob_id(main.as_bytes()),"bytes":main.len()});
    let delta = rpc(
        app,
        bearer,
        "index_repository",
        json!({"repo":repo,"commit":next,"files":changed}),
    )
    .await?;
    assert_eq!(
        delta["structuredContent"]["missing"],
        json!([blob_id(main.as_bytes())]),
        "{delta}"
    );
    let overview = rpc(
        app,
        bearer,
        "repo_overview",
        json!({"repo":short,"commit":commit}),
    )
    .await?;
    assert_eq!(
        overview["structuredContent"]["summary"]["commands"]["test"], "npm run test",
        "{overview}"
    );
    assert!(
        overview["structuredContent"]["memories"]
            .as_array()
            .is_some_and(|m| !m.is_empty()),
        "overview includes repository memory"
    );
    let listed = rpc(app, bearer, "list_code_indexes", json!({"repo":short})).await?;
    assert_eq!(
        listed["structuredContent"]["snapshots"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    let deleted = rpc(
        app,
        bearer,
        "delete_code_index",
        json!({"repo":short,"commit":next}),
    )
    .await?;
    assert_eq!(deleted["structuredContent"]["deleted_snapshots"], 1);
    Ok(())
}

async fn raw(
    app: &Router,
    method: &str,
    path: &str,
    body: String,
    headers: &[(&str, &str)],
) -> Result<axum::response::Response> {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .header("host", "127.0.0.1:3001")
        .body(Body::from(body))?;
    request
        .extensions_mut()
        .insert(ConnectInfo("192.0.2.9:4000".parse::<SocketAddr>()?));
    for (name, value) in headers {
        request.headers_mut().append(
            (*name).parse::<axum::http::HeaderName>()?,
            HeaderValue::from_str(value)?,
        );
    }
    Ok(app.clone().oneshot(request).await?)
}
async fn authorize(
    app: &Router,
    session: &[(&str, &str)],
    client: &str,
    challenge: &str,
    decision: &str,
) -> Result<(StatusCode, Value)> {
    attack(
        app.clone(),
        "POST",
        "/v1/oauth/authorize",
        json!({"client_id":client,"redirect_uri":"http://127.0.0.1:8123/callback","response_type":"code","code_challenge":challenge,"code_challenge_method":"S256","scope":"capabilities:read memory:read","state":"xyz","resource":"http://127.0.0.1:3001/mcp","decision":decision}).to_string(),
        session,
    )
    .await
}
fn code_from(redirect: &Value) -> Result<String> {
    let url = reqwest::Url::parse(redirect.as_str().ok_or("missing redirect")?)?;
    let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(pairs.get("state").map(String::as_str), Some("xyz"));
    assert_eq!(
        pairs.get("iss").map(String::as_str),
        Some("http://127.0.0.1:3001")
    );
    Ok(pairs.get("code").cloned().ok_or("missing code")?)
}
async fn token(app: &Router, body: Value) -> Result<(StatusCode, Value)> {
    attack(app.clone(), "POST", "/oauth/token", body.to_string(), &[]).await
}
#[allow(clippy::too_many_lines)]
async fn oauth(
    app: &Router,
    api_key: &str,
    session: &[(&str, &str)],
    state: &hiveway::state::AppState,
) -> Result {
    let unauthenticated = raw(
        app,
        "POST",
        "/mcp",
        json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
        &[],
    )
    .await?;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
    let challenge_header = unauthenticated
        .headers()
        .get("www-authenticate")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert!(
        challenge_header.contains(
            "resource_metadata=\"http://127.0.0.1:3001/.well-known/oauth-protected-resource/mcp\""
        ),
        "{challenge_header}"
    );
    let (status, resource) = attack(
        app.clone(),
        "GET",
        "/.well-known/oauth-protected-resource/mcp",
        String::new(),
        &[],
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        resource["authorization_servers"][0],
        "http://127.0.0.1:3001"
    );
    let (_, metadata) = attack(
        app.clone(),
        "GET",
        "/.well-known/oauth-authorization-server",
        String::new(),
        &[],
    )
    .await?;
    assert_eq!(
        metadata["code_challenge_methods_supported"],
        json!(["S256"])
    );
    assert_eq!(metadata["client_id_metadata_document_supported"], true);
    let (status, rejected) = attack(
        app.clone(),
        "POST",
        "/oauth/register",
        json!({"client_name":"Bad","redirect_uris":["http://evil.example/cb"]}).to_string(),
        &[],
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{rejected}");
    let (status, client) = attack(app.clone(), "POST", "/oauth/register", json!({"client_name":"Platform test","redirect_uris":["http://127.0.0.1/callback"],"application_type":"native"}).to_string(), &[]).await?;
    assert_eq!(status, StatusCode::CREATED, "{client}");
    let client = client["client_id"]
        .as_str()
        .ok_or("missing client id")?
        .to_owned();
    let verifier = "v".repeat(50);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let key_header = [("authorization", api_key)];
    let (status, _) = authorize(app, &key_header, &client, &challenge, "inspect").await?;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "API keys cannot approve OAuth clients"
    );
    let (status, consent) = authorize(app, session, &client, &challenge, "inspect").await?;
    assert_eq!(status, StatusCode::OK, "{consent}");
    assert_eq!(consent["scopes"].as_array().map(Vec::len), Some(2));
    assert_eq!(consent["client"]["name"], "Platform test");
    let (_, approved) = authorize(app, session, &client, &challenge, "approve").await?;
    let code = code_from(&approved["redirect"])?;
    let exchange = json!({"grant_type":"authorization_code","code":code,"client_id":client,"redirect_uri":"http://127.0.0.1:8123/callback","code_verifier":verifier});
    let (status, wrong) = token(app, json!({"grant_type":"authorization_code","code":code,"client_id":client,"redirect_uri":"http://127.0.0.1:8123/callback","code_verifier":"w".repeat(50)})).await?;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "PKCE must bind the code: {wrong}"
    );
    let (status, tokens) = token(app, exchange.clone()).await?;
    assert_eq!(status, StatusCode::OK, "{tokens}");
    assert_eq!(tokens["scope"], "capabilities:read memory:read");
    let access = format!(
        "Bearer {}",
        tokens["access_token"]
            .as_str()
            .ok_or("missing access token")?
    );
    let (status, listed) = attack(
        app.clone(),
        "POST",
        "/mcp",
        json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
        &[("authorization", &access)],
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let names: Vec<&str> = listed["result"]["tools"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(
        names.contains(&"recall") && !names.contains(&"remember"),
        "{names:?}"
    );
    let (status, _) = attack(
        app.clone(),
        "POST",
        "/v1/account/credentials",
        json!({"name":"escalate"}).to_string(),
        &[("authorization", &access)],
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "OAuth tokens cannot mint API keys"
    );
    let (_, grants) = attack(
        app.clone(),
        "GET",
        "/v1/account/oauth/grants",
        String::new(),
        session,
    )
    .await?;
    assert_eq!(
        grants["grants"][0]["client_name"], "Platform test",
        "{grants}"
    );
    let refresh = tokens["refresh_token"].clone();
    let (status, rotated) = token(
        app,
        json!({"grant_type":"refresh_token","refresh_token":refresh,"client_id":client}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{rotated}");
    let rotated_access = format!(
        "Bearer {}",
        rotated["access_token"]
            .as_str()
            .ok_or("missing rotated token")?
    );
    let (status, reuse) = token(
        app,
        json!({"grant_type":"refresh_token","refresh_token":refresh,"client_id":client}),
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{reuse}");
    let (status, _) = attack(
        app.clone(),
        "POST",
        "/mcp",
        json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
        &[("authorization", &rotated_access)],
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "refresh token reuse revokes the grant"
    );
    // A replayed authorization code revokes what its first redemption issued.
    let (_, approved) = authorize(app, session, &client, &challenge, "approve").await?;
    let code = code_from(&approved["redirect"])?;
    let mut exchange = exchange;
    exchange["code"] = json!(code);
    let (_, second) = token(app, exchange.clone()).await?;
    let second_access = format!(
        "Bearer {}",
        second["access_token"]
            .as_str()
            .ok_or("missing second token")?
    );
    let (status, _) = token(app, exchange).await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = attack(
        app.clone(),
        "POST",
        "/mcp",
        json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
        &[("authorization", &second_access)],
    )
    .await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (_, denied) = authorize(app, session, &client, &challenge, "deny").await?;
    assert!(
        denied["redirect"]
            .as_str()
            .is_some_and(|r| r.contains("error=access_denied"))
    );
    hiveway::features::oauth::cleanup(state).await?;
    Ok(())
}
