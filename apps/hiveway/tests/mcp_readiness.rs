//! Regression coverage for real developer discovery and account-scoped execution history.
#![cfg(not(miri))]
use axum::{
    Router,
    body::{Body, to_bytes},
    http::Request,
};
use hiveway::{
    entities::agent_credentials,
    features::{common::settings::Settings, identity},
    state::AppState,
};
use sea_orm::{ActiveModelTrait, ConnectionTrait, Database, DbBackend, Set, Statement};
use serde_json::{Value, json};
use std::io::Write;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use uuid::Uuid;
#[derive(Clone)]
struct LogBuffer(Arc<Mutex<Vec<u8>>>);
impl Write for LogBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|_| std::io::Error::other("log buffer poisoned"))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const KEY: &str = "hwa_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const PEER: &str = "hwa_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const BROWSER: &str = "hwa_cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const OTHER: &str = "hwa_dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
async fn request(app: &Router, key: &str, path: &str, body: Option<Value>) -> Result<Value> {
    let mut r = Request::builder()
        .uri(path)
        .header("authorization", format!("Bearer {key}"));
    if body.is_some() {
        r = r.method("POST").header("content-type", "application/json");
    }
    let mut built = r.body(Body::from(body.map_or_else(String::new, |v| v.to_string())))?;
    built.headers_mut().insert(
        "host",
        axum::http::HeaderValue::from_static("127.0.0.1:3001"),
    );
    built.extensions_mut().insert(axum::extract::ConnectInfo(
        "192.0.2.10:4000".parse::<std::net::SocketAddr>()?,
    ));
    let response = app.clone().oneshot(built).await?;
    assert!(response.headers().contains_key("x-request-id"));
    assert!(response.status().is_success(), "{}", response.status());
    Ok(serde_json::from_slice(
        &to_bytes(response.into_body(), 2_000_000).await?,
    )?)
}
async fn call(app: &Router, key: &str, name: &str, args: Value) -> Result<Value> {
    let r=request(app,key,"/mcp",Some(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"hive/client":{"agent":"regression","harness":"test"}}}}))).await?;
    Ok(r["result"].clone())
}
async fn discovery(app: &Router) -> Result {
    // Replay all 56 advertised provider descriptions from the live audit. Schemas are
    // reduced to objects because these tests exercise discovery, not upstream APIs.
    let providers: serde_json::Map<String, Value> =
        serde_json::from_str(include_str!("fixtures/provider-discovery.json"))?;
    for (name, tools) in providers {
        call(
            app,
            KEY,
            "register_mcp_connection",
            json!({"name":name,"tools":tools}),
        )
        .await?;
    }
    for query in [
        "How do I implement authentication middleware in Next.js?",
        "Add Stripe subscriptions to my Next.js app",
        "Next.js library documentation authentication middleware",
    ] {
        let r = call(
            app,
            KEY,
            "search_tools",
            json!({"query":query,"stack":["nextjs","typescript"]}),
        )
        .await?;
        let content = &r["structuredContent"];
        assert_eq!(
            content["tools"][0]["name"], "mcp_context7__resolve-library-id",
            "{r}"
        );
        assert!(
            !content["tools"]
                .as_array()
                .ok_or("tools missing")?
                .iter()
                .any(|v| v["_meta"]["dev.hive/requiresBinding"] == true)
        );
    }
    let r = call(
        app,
        KEY,
        "search_tools",
        json!({"query":"web search recent React security advisories"}),
    )
    .await?;
    assert_eq!(
        r["structuredContent"]["tools"][0]["name"], "mcp_tinyfish__search",
        "{r}"
    );
    let mixed=call(app,KEY,"search_tools",json!({"query":"Research Next.js deployment pitfalls on the web and look up current framework documentation"})).await?;
    let mixed = mixed["structuredContent"]["tools"]
        .as_array()
        .ok_or("missing tools")?;
    assert!(
        mixed
            .iter()
            .any(|t| t["name"] == "mcp_context7__resolve-library-id")
    );
    assert!(mixed.iter().any(|t| t["name"] == "mcp_tinyfish__search"));
    let design=call(app,KEY,"search_tools",json!({"query":"Build a website for a coffee shop, find design inspiration and preview templates"})).await?;
    assert_eq!(
        design["structuredContent"]["tools"][0]["name"],
        "mcp_templr__search_templates"
    );
    let r = call(
        app,
        KEY,
        "search_tools",
        json!({"query":"Analyze CSV using SQL"}),
    )
    .await?;
    assert!(
        !r["structuredContent"]["tools"]
            .as_array()
            .ok_or("missing tools")?
            .iter()
            .any(|v| v["name"] == "workflow_dataset_profile")
    );
    assert!(
        r["structuredContent"]["available_to_enable"]
            .as_array()
            .ok_or("missing hints")?
            .iter()
            .any(|v| v["slug"] == "duckdb" && v["shared_status"] == "offline")
    );
    Ok(())
}
async fn check(app: &Router, state: &AppState) -> Result {
    // Native discovered tools follow the same call_tool contract as routed providers.
    let recalled = call(
        app,
        KEY,
        "call_tool",
        json!({"name":"recall","arguments":{"repo":"global","query":"secret-audit-canary"}}),
    )
    .await?;
    assert_eq!(recalled["isError"], false, "{recalled}");
    let rejected = call(
        app,
        KEY,
        "call_tool",
        json!({"name":"disconnect_mcp","arguments":{}}),
    )
    .await?;
    assert_eq!(rejected["isError"], true);
    call(app, PEER, "recall", json!({"repo":"global"})).await?;
    call(app, BROWSER, "recall", json!({"repo":"global"})).await?;
    call(app, OTHER, "recall", json!({"repo":"global"})).await?;
    let own = request(app, KEY, "/v1/activity", None).await?;
    let peer = request(app, PEER, "/v1/activity", None).await?;
    let browser = request(app, BROWSER, "/v1/activity", None).await?;
    let other = request(app, OTHER, "/v1/activity", None).await?;
    assert_eq!(peer["tool_counts"]["total"], 1);
    assert_eq!(other["tool_counts"]["total"], 1);
    assert_eq!(
        browser["tool_counts"]["total"].as_u64(),
        own["tool_counts"]["total"].as_u64().map(|n| n + 2)
    );
    assert!(
        own["tool_calls"]
            .as_array()
            .ok_or("missing events")?
            .iter()
            .any(|v| v["tool"] == "recall" && v["state"] == "succeeded")
    );
    assert!(
        own["tool_calls"]
            .as_array()
            .ok_or("missing events")?
            .iter()
            .any(|v| v["state"] == "failed")
    );
    assert!(
        !own.to_string().contains("secret-audit-canary"),
        "arguments must not enter the ledger"
    );
    assert_eq!(own["tool_calls"][0]["client_context"]["harness"], "test");
    // A failed persisted workflow must be a semantic MCP error and appear as failed activity.
    let run=call(app,KEY,"run_workflow",json!({"definition":{"steps":[{"id":"missing","tool":"mcp_duckdb__execute_query","arguments":{"query":"SELECT 1"}}]},"wait_seconds":5})).await?;
    assert_eq!(run["isError"], true, "{run}");
    assert_eq!(run["structuredContent"]["status"], "failed");
    let history = request(app, KEY, "/v1/activity", None).await?;
    assert!(
        history["tool_calls"]
            .as_array()
            .ok_or("missing events")?
            .iter()
            .any(|v| v["tool"] == "mcp_duckdb__execute_query"
                && v["run_id"].is_string()
                && v["state"] == "failed")
    );
    task_reconciliation(state).await?;
    shared_readiness(state).await?;
    upstream_error_privacy(state).await?;
    reconciliation(state, &run["structuredContent"]["run_id"]).await?;
    pagination(app, state).await
}
async fn shared_readiness(state: &AppState) -> Result {
    let statuses = hiveway::features::connections::required_shared_health(state).await?;
    assert!(
        statuses
            .iter()
            .any(|v| v["name"] == "duckdb" && v["online"] == false)
    );
    let mut required = Settings::from_env()?;
    required.required_shared_integrations = vec!["duckdb".into()];
    let mut required_state = state.clone();
    required_state.settings = Arc::new(required);
    let error = hiveway::features::connections::required_shared_health(&required_state)
        .await
        .err()
        .ok_or("required offline service must block readiness")?;
    assert_eq!(error.code, "required_shared_integration_offline");
    Ok(())
}
async fn upstream_error_privacy(state: &AppState) -> Result {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("authorization", format!("Bearer {KEY}").parse()?);
    let credential = identity::authenticate(state, &headers, "").await?;
    let activity = hiveway::features::tool_activity::Call::start(
        state,
        &credential,
        "mcp_tinyfish__search",
        &headers,
    )
    .await;
    activity
        .finish(
            state,
            &json!({"isError":true,"structuredContent":{"error":"secret-audit-canary"}}),
            None,
        )
        .await;
    let rows = hiveway::features::tool_activity::list(state, &credential, false, None).await?;
    assert!(!serde_json::to_string(&rows)?.contains("secret-audit-canary"));
    Ok(())
}
async fn reconciliation(state: &AppState, run: &Value) -> Result {
    let run: Uuid = run.as_str().ok_or("missing run id")?.parse()?;
    let accepted = Uuid::new_v4();
    let abandoned = Uuid::new_v4();
    for (id, status, run, deadline) in [
        (accepted, "accepted", Some(run), i64::MAX),
        (abandoned, "running", None, 0),
    ] {
        state.postgres.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO mcp_activity(id,owner,credential_id,tool,client_context,run_id,state,created_at,deadline,expires_at) SELECT $1,wallet,id,'run_workflow','{}',$2,$3,0,$4,3000000000 FROM agent_credentials WHERE token_hash=$5",
            vec![id.into(),run.into(),status.into(),deadline.into(),identity::hash(KEY.as_bytes()).into()],
        )).await?;
    }
    hiveway::features::tool_activity::cleanup(state).await?;
    for (id, expected) in [(accepted, "failed"), (abandoned, "interrupted")] {
        let row = state
            .postgres
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT state FROM mcp_activity WHERE id=$1",
                vec![id.into()],
            ))
            .await?
            .ok_or("missing activity")?;
        assert_eq!(row.try_get::<String>("", "state")?, expected);
    }
    Ok(())
}
async fn task_reconciliation(state: &AppState) -> Result {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("authorization", format!("Bearer {KEY}").parse()?);
    let credential = identity::authenticate(state, &headers, "").await?;
    for (failed, expected) in [(false, "succeeded"), (true, "failed")] {
        let task = Uuid::new_v4();
        state.postgres.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO mcp_jobs(id,connection_id,request,expires_at,caller,status,result,created_at,updated_at) SELECT $1,id,'{}',3000000000,owner,'completed',$2,0,1 FROM mcp_connections WHERE owner=$3 AND name='tinyfish'",
            vec![task.into(),json!({"isError":failed}).into(),credential.wallet.clone().into()],
        )).await?;
        let call = hiveway::features::tool_activity::Call::start(
            state,
            &credential,
            "mcp_tinyfish__search",
            &headers,
        )
        .await;
        let id = call.id;
        call.finish(
            state,
            &json!({"structuredContent":{"task":{"taskId":task,"status":"queued"}}}),
            None,
        )
        .await;
        hiveway::features::tool_activity::cleanup(state).await?;
        let rows = hiveway::features::tool_activity::list(state, &credential, false, None).await?;
        assert!(rows.iter().any(|v| v["id"] == id.to_string()
            && v["state"] == expected
            && v["task_id"] == task.to_string()));
    }
    Ok(())
}
async fn pagination(app: &Router, state: &AppState) -> Result {
    state.postgres.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO mcp_activity(id,owner,credential_id,tool,client_context,state,created_at,deadline,expires_at) SELECT gen_random_uuid(),'github:7001',id,'recall','{}','succeeded',2000000000,2000000001,3000000000 FROM agent_credentials CROSS JOIN generate_series(1,105) WHERE token_hash=$1",
        vec![identity::hash(KEY.as_bytes()).into()],
    )).await?;
    let mut path = "/v1/activity".to_owned();
    let mut ids = std::collections::HashSet::new();
    let mut expected = 0;
    for page in 0..4 {
        let response = request(app, KEY, &path, None).await?;
        if page == 0 {
            expected = response["tool_counts"]["total"]
                .as_u64()
                .ok_or("missing count")?;
        } else {
            assert!(response.get("tool_counts").is_none());
        }
        for event in response["tool_calls"].as_array().ok_or("missing page")? {
            assert!(
                ids.insert(event["id"].as_str().ok_or("missing id")?.to_owned()),
                "duplicate activity across pages"
            );
        }
        let cursor = &response["next_cursor"];
        if cursor.is_null() {
            break;
        }
        path = format!(
            "/v1/activity?before_time={}&before_id={}",
            cursor["before_time"],
            cursor["before_id"].as_str().ok_or("missing cursor")?
        );
    }
    assert_eq!(
        u64::try_from(ids.len())?,
        expected,
        "pagination must not lose tied timestamps"
    );
    Ok(())
}
#[tokio::test(flavor = "current_thread")]
#[ignore = "requires HIVE_TEST_DATABASE_URL pointing at disposable PostgreSQL"]
async fn developer_discovery_and_activity_are_reliable() -> Result {
    let logs = LogBuffer(Arc::new(Mutex::new(Vec::new())));
    let writer = logs.clone();
    let subscriber = tracing_subscriber::fmt()
        .json()
        .with_writer(move || writer.clone())
        .finish();
    let _subscriber = tracing::subscriber::set_default(subscriber);
    let admin = Database::connect(std::env::var("HIVE_TEST_DATABASE_URL")?).await?;
    let schema = format!("mcp_readiness_{}", Uuid::new_v4().simple());
    admin
        .execute_unprepared(&format!("CREATE SCHEMA {schema}"))
        .await?;
    let mut options = sea_orm::ConnectOptions::new(std::env::var("HIVE_TEST_DATABASE_URL")?);
    options
        .set_schema_search_path(schema.clone())
        .sqlx_logging(false);
    let db = Database::connect(options).await?;
    let outcome = async {
        hiveway::database::migrate(&db).await?;
        let settings = Settings::from_env()?;
        let state = AppState {
            sandbox: Arc::new(hiveway::features::cell::runtime::Sandbox::new(
                settings.runtime.sandbox.clone(),
            )),
            settings: Arc::new(settings),
            postgres: db.clone(),
            redis: fred::prelude::Builder::default_centralized().build()?,
            http: reqwest::Client::new(),
            realtime: hiveway::features::realtime::Realtime::default(),
        };
        for (key, owner, account_key) in [
            (KEY, "github:7001", true),
            (PEER, "github:7001", true),
            (BROWSER, "github:7001", false),
            (OTHER, "github:7002", true),
        ] {
            let id = Uuid::new_v4();
            agent_credentials::ActiveModel {
                id: Set(id),
                wallet: Set(owner.into()),
                name: Set("test agent".into()),
                token_hash: Set(identity::hash(key.as_bytes())),
                scopes: Set(vec![
                    "capabilities:invoke",
                    "capabilities:read",
                    "capabilities:publish",
                    "receipts:read",
                    "memory:read",
                    "code:read",
                ]
                .into_iter()
                .map(str::to_owned)
                .collect()),
                per_call_limit: Set(1),
                total_limit: Set(1),
                expires_at: Set(i64::MAX),
                created_at: Set(0),
                ..Default::default()
            }
            .insert(&db)
            .await?;
            if account_key {
                hiveway::entities::account_api_keys::ActiveModel {
                    id: Set(id),
                    owner: Set(owner.into()),
                }
                .insert(&db)
                .await?;
            }
        }
        hiveway::features::integrations::sync(&db).await?;
        hiveway::features::hiveway::workflows::sync_official(&state).await?;
        let (app, _drain) = hiveway::api::protect(
            hiveway::api::router().with_state(state.clone()),
            state.clone(),
        );
        discovery(&app).await?;
        check(&app, &state).await?;
        let logged = String::from_utf8(logs.0.lock().map_err(|_| "log buffer poisoned")?.clone())?;
        for marker in [
            "HTTP request finished",
            "MCP tool call finished",
            "mcp.workflow_tool",
            "request_id",
            "call_id",
        ] {
            assert!(logged.contains(marker), "missing log field/event {marker}");
        }
        assert!(!logged.contains("secret-audit-canary"));
        Ok(())
    }
    .await;
    db.close().await?;
    admin
        .execute_unprepared(&format!("DROP SCHEMA {schema} CASCADE"))
        .await?;
    admin.close().await?;
    outcome
}
