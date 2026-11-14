//! Exercises the real MCP gateway and persisted executor in an isolated schema.
#![cfg(not(miri))]
use axum::{
    Router,
    body::{Body, to_bytes},
    http::Request,
};
use hiveway::{
    entities::agent_credentials,
    features::{common::settings::Settings, hiveway::workflows, identity},
    state::AppState,
};
use sea_orm::{ActiveModelTrait, ConnectionTrait, Database, Set};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

async fn rpc(app: &Router, token: &str, name: &str, args: Value) -> Result<Value> {
    let response = app.clone().oneshot(Request::post("/mcp")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        // A forged client depth must not bypass run records and budgets.
        .header("x-hive-workflow-depth", "1")
        .body(Body::from(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args}}).to_string()))?).await?;
    let body: Value = serde_json::from_slice(&to_bytes(response.into_body(), 2_000_000).await?)?;
    assert!(body.get("error").is_none(), "{body}");
    Ok(body["result"].clone())
}

#[tokio::test]
#[ignore = "requires HIVE_TEST_DATABASE_URL pointing at disposable PostgreSQL"]
async fn orchestrates_native_and_connected_tools_and_publishes_patterns() -> Result {
    let url = std::env::var("HIVE_TEST_DATABASE_URL")?;
    let admin = Database::connect(url.clone()).await?;
    let schema = format!("workflow_test_{}", Uuid::new_v4().simple());
    admin
        .execute_unprepared(&format!("CREATE SCHEMA {schema}"))
        .await?;
    let mut options = sea_orm::ConnectOptions::new(url);
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
        for (token, owner, scopes) in [
            (
                "hwa_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "github:988887",
                vec![
                    "capabilities:invoke",
                    "capabilities:publish",
                    "memory:read",
                    "memory:write",
                    "workflows:publish",
                ],
            ),
            (
                "hwa_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "github:988888",
                vec!["capabilities:invoke", "memory:read"],
            ),
        ] {
            let id = Uuid::new_v4();
            agent_credentials::ActiveModel {
                id: Set(id),
                wallet: Set(owner.into()),
                name: Set("fixture".into()),
                token_hash: Set(identity::hash(token.as_bytes())),
                scopes: Set(scopes.into_iter().map(str::to_owned).collect()),
                per_call_limit: Set(1),
                total_limit: Set(1),
                expires_at: Set(i64::MAX),
                created_at: Set(0),
                ..Default::default()
            }
            .insert(&db)
            .await?;
            hiveway::entities::account_api_keys::ActiveModel {
                id: Set(id),
                owner: Set(owner.into()),
            }
            .insert(&db)
            .await?;
        }
        hiveway::features::integrations::sync(&db).await?;
        workflows::sync_official(&state).await?;
        workflows::sync_official(&state).await?;
        let app = hiveway::api::router().with_state(state.clone());
        exercise(&app, &state).await?;
        discovery_led_chain(&app).await
    }
    .await;
    db.close().await?;
    admin
        .execute_unprepared(&format!("DROP SCHEMA {schema} CASCADE"))
        .await?;
    admin.close().await?;
    outcome
}

#[allow(clippy::too_many_lines)]
async fn exercise(app: &Router, state: &AppState) -> Result {
    let token = "hwa_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let definition = json!({
        "inputs":{"type":"object","properties":{"title":{"type":"string"}},"required":["title"]},
        "steps":[
            {"id":"save","tool":"remember","arguments":{"repo":"global","kind":"note","title":"{{inputs.title}}","body":"A durable workflow fixture."}},
            {"id":"recall","tool":"recall","after":["save"],"arguments":{"repo":"global","query":"{{inputs.title}}"}},
            {"id":"skip","tool":"remember","when":"{{inputs.missing}}","arguments":{}},
            {"id":"optional","tool":"mcp_missing__tool","on_error":"continue","arguments":{}}
        ],
        "output":{"memories":"{{steps.recall.json.memories}}","optional_ok":"{{steps.optional.ok}}"}
    });
    let dry = rpc(
        app,
        token,
        "run_workflow",
        json!({"definition":definition,"inputs":{"title":"Workflow fixture"},"dry_run":true}),
    )
    .await?;
    assert_eq!(dry["structuredContent"]["valid"], true, "{dry}");
    let run = rpc(
        app,
        token,
        "run_workflow",
        json!({"definition":definition,"inputs":{"title":"Workflow fixture"},"wait_seconds":5,"include_steps":true}),
    )
    .await?;
    let run = &run["structuredContent"];
    assert_eq!(run["status"], "completed", "{run}");
    assert!(
        run["run_id"].is_string(),
        "client depth cannot force inline execution"
    );
    assert_eq!(
        run["output"]["memories"][0]["title"], "Workflow fixture",
        "{run}"
    );
    assert_eq!(run["output"]["optional_ok"], false);
    assert_eq!(run["steps"]["skip"]["status"], "skipped");
    assert_eq!(run["tool_calls"], 3);
    let compact = rpc(
        app,
        token,
        "get_workflow_run",
        json!({"run_id":run["run_id"]}),
    )
    .await?;
    assert!(
        compact["structuredContent"].get("steps").is_none(),
        "{compact}"
    );
    assert_eq!(compact["structuredContent"]["output"], run["output"]);
    let diagnostic = rpc(
        app,
        token,
        "get_workflow_run",
        json!({"run_id":run["run_id"],"include_steps":true}),
    )
    .await?;
    assert_eq!(diagnostic["structuredContent"]["steps"], run["steps"]);
    let other = rpc(
        app,
        "hwa_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "get_workflow_run",
        json!({"run_id":run["run_id"],"wait_seconds":0}),
    )
    .await?;
    assert_eq!(other["isError"], true, "another account cannot read runs");
    let forbidden = rpc(app, "hwa_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "run_workflow", json!({"definition":{"steps":[{"id":"write","tool":"remember","arguments":{"repo":"global","kind":"note","title":"Unauthorized","body":"must not be saved"}}]},"wait_seconds":5})).await?;
    assert_eq!(
        forbidden["structuredContent"]["status"], "failed",
        "step scopes must be enforced: {forbidden}"
    );

    // Connected tools and native memory share the same dispatcher and result templates.
    let registered = rpc(app, token, "register_mcp_connection", json!({"name":"workflow-fixture","tools":[{"name":"lookup","description":"fixture","inputSchema":{"type":"object","properties":{}},"annotations":{"readOnlyHint":true}}]})).await?;
    let connection = registered["structuredContent"]["id"]
        .as_str()
        .ok_or("missing connection")?
        .to_owned();
    let worker_app = app.clone();
    let worker_connection = connection.clone();
    let worker = tokio::spawn(async move {
        let job = rpc(
            &worker_app,
            token,
            "poll_mcp",
            json!({"connection_id":worker_connection,"wait_seconds":8}),
        )
        .await
        .map_err(|e| e.to_string())?;
        let job = &job["structuredContent"]["job"];
        let completed = rpc(&worker_app, token, "complete_mcp", json!({"connection_id":worker_connection,"job_id":job["id"],"lease":job["lease"],"result":{"content":[{"type":"text","text":"Connected result fixture"}]}})).await.map_err(|e|e.to_string())?;
        assert_eq!(completed["isError"], false, "{completed}");
        Ok::<(), String>(())
    });
    let connected = rpc(app, token, "run_workflow", json!({"definition":{"steps":[
        {"id":"fetch","tool":"mcp_workflow-fixture__lookup","arguments":{}},
        {"id":"save","tool":"remember","arguments":{"repo":"global","kind":"note","title":"Connected workflow","body":"{{steps.fetch.text}}"}}
    ],"output":"{{steps.save.json.memory.body}}"},"wait_seconds":5})).await?;
    worker.await?.map_err(|e| format!("worker: {e}"))?;
    assert!(connected["structuredContent"].get("steps").is_none());
    assert_eq!(
        connected["structuredContent"]["output"], "Connected result fixture",
        "{connected}"
    );

    let published = rpc(app, token, "publish_workflow", json!({"name":"Nested fixture","description":"Calls memory","definition":{"steps":[{"id":"read","tool":"recall","arguments":{"repo":"global"}}]}})).await?;
    let tool = published["structuredContent"]["published"]["tool"]
        .as_str()
        .ok_or("missing published tool")?;
    let nested = rpc(app, token, "run_workflow", json!({"definition":{"steps":[{"id":"nested","tool":tool,"arguments":{}}]},"wait_seconds":5})).await?;
    assert_eq!(
        nested["structuredContent"]["status"], "completed",
        "{nested}"
    );
    assert_eq!(
        nested["structuredContent"]["tool_calls"], 2,
        "nested calls share the root budget"
    );
    assert!(
        nested["structuredContent"]["output"].get("steps").is_none(),
        "{nested}"
    );
    let mut constrained = state.clone();
    let mut settings = Settings::from_env()?;
    settings.runtime.workflows.max_tool_calls = 1;
    constrained.settings = Arc::new(settings);
    let limited_app = hiveway::api::router().with_state(constrained);
    let limited=rpc(&limited_app,token,"run_workflow",json!({"definition":{"steps":[{"id":"nested","tool":tool,"arguments":{}}]},"wait_seconds":5})).await?;
    assert_eq!(
        limited["structuredContent"]["status"], "failed",
        "{limited}"
    );
    assert_eq!(limited["structuredContent"]["tool_calls"], 1);
    let failed_output=rpc(app,token,"run_workflow",json!({"definition":{"steps":[{"id":"read","tool":"recall","arguments":{"repo":"global"}}],"output":"{{inputs.value | truncate: invalid}}"},"wait_seconds":5})).await?;
    assert_eq!(
        failed_output["structuredContent"]["status"], "failed",
        "output errors must not be reported as success: {failed_output}"
    );

    for (bad, code) in [
        (
            json!({"steps":[{"id":"a","tool":"recall","arguments":{"query":"{{steps.b.text}}"}},{"id":"b","tool":"recall"}]}),
            "invalid_workflow_reference",
        ),
        (
            json!({"steps":[{"id":"a","tool":"disconnect_mcp"}]}),
            "workflow_step_tool_not_allowed",
        ),
        (
            json!({"inputs":{"type":"object","properties":{"value":{"$ref":"file:///etc/passwd"}}},"steps":[{"id":"read","tool":"recall"}]}),
            "invalid_workflow_inputs_schema",
        ),
    ] {
        let result = rpc(
            app,
            token,
            "run_workflow",
            json!({"definition":bad,"dry_run":true}),
        )
        .await?;
        assert_eq!(result["structuredContent"]["error"], code, "{result}");
    }
    let foreach = rpc(app, token, "run_workflow", json!({"definition":{"steps":[{"id":"items","tool":"recall","for_each":"{{inputs.items}}","when":"{{item.enabled}}","arguments":{"repo":"global"}}]},"inputs":{"items":[{"enabled":false},{"enabled":true}]},"wait_seconds":5})).await?;
    assert_eq!(
        foreach["structuredContent"]["status"], "completed",
        "{foreach}"
    );
    assert_eq!(foreach["structuredContent"]["tool_calls"], 1);
    cancellation(app, state, &connection).await?;

    // Direct pattern recording verifies threshold, privacy, defaults and later variation.
    for n in 0..=state.settings.runtime.workflows.auto_publish_after {
        let definition: workflows::Definition = serde_json::from_value(
            json!({"inputs":{"type":"object","properties":{"query":{"type":"string","default":"fixture"}}},"steps":[
                {"id":"one","tool":"recall","arguments":{"repo":"global","query":"{{inputs.query}}"},"on_error":"continue"},
                {"id":"two","tool":"recall","arguments":{"repo":if n < state.settings.runtime.workflows.auto_publish_after {"global"} else {"github.com/acme/demo"}}}
            ]}),
        )?;
        workflows::patterns::record(state, "github:pattern-fixture", &definition).await?;
    }
    let row = state.postgres.query_one_raw(sea_orm::Statement::from_string(sea_orm::DbBackend::Postgres,"SELECT definition,visibility FROM hive_workflows WHERE owner='github:pattern-fixture' AND origin='auto'" )).await?.ok_or("missing automatic workflow")?;
    let auto: Value = row.try_get("", "definition")?;
    assert_eq!(row.try_get::<String>("", "visibility")?, "private");
    assert_eq!(auto["inputs"]["properties"]["query"]["default"], "fixture");
    assert_eq!(auto["steps"][0]["on_error"], "continue");
    assert_eq!(auto["steps"][1]["arguments"]["repo"], "{{inputs.two_repo}}");
    Ok(())
}

async fn cancellation(app: &Router, state: &AppState, connection: &str) -> Result {
    let token = "hwa_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let started=rpc(app,token,"run_workflow",json!({"definition":{"steps":[
        {"id":"wait","tool":"mcp_workflow-fixture__lookup","arguments":{"unique":"cancel-fixture"}},
        {"id":"never","tool":"remember","after":["wait"],"arguments":{"repo":"global","kind":"note","title":"Must not run","body":"cancelled"}}
    ]},"wait_seconds":0})).await?;
    let run_id = started["structuredContent"]["run_id"]
        .as_str()
        .ok_or("missing run id")?
        .parse::<Uuid>()?;
    let claimed = rpc(
        app,
        token,
        "poll_mcp",
        json!({"connection_id":connection,"wait_seconds":5}),
    )
    .await?;
    let job = &claimed["structuredContent"]["job"];
    assert!(job.is_object(), "{claimed}");
    let cancelled = rpc(app, token, "cancel_workflow_run", json!({"run_id":run_id})).await?;
    assert_eq!(
        cancelled["structuredContent"]["cancellation_requested"], true,
        "{cancelled}"
    );
    rpc(app,token,"complete_mcp",json!({"connection_id":connection,"job_id":job["id"],"lease":job["lease"],"result":{"content":[{"type":"text","text":"finished in-flight step"}]}})).await?;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let row = state
            .postgres
            .query_one_raw(sea_orm::Statement::from_sql_and_values(
                sea_orm::DbBackend::Postgres,
                "SELECT steps FROM workflow_runs WHERE id=$1",
                vec![run_id.into()],
            ))
            .await?
            .ok_or("missing run")?;
        let steps: Value = row.try_get("", "steps")?;
        if steps["wait"]["status"] == "completed" {
            assert!(steps.get("never").is_none());
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "cancelled run did not settle"
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    Ok(())
}

async fn discovery_led_chain(app: &Router) -> Result {
    let token = "hwa_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let source=rpc(app,token,"register_mcp_connection",json!({"name":"smart-source","tools":[{"name":"lookup","description":"qa_dynamic_producer URL records","inputSchema":{"type":"object"}}]})).await?;
    let sink=rpc(app,token,"register_mcp_connection",json!({"name":"smart-sink","tools":[{"name":"fetch","description":"qa_dynamic_consumer URL reader","inputSchema":{"type":"object","properties":{"url":{"type":"string"},"sequence":{"type":"integer"}},"required":["url","sequence"]}}]})).await?;
    let find = |query| rpc(app, token, "search_tools", json!({"query":query}));
    let producers = find("qa_dynamic_producer").await?;
    let consumers = find("qa_dynamic_consumer").await?;
    let source_name = producers["structuredContent"]["tools"]
        .as_array()
        .ok_or("missing producer results")?
        .iter()
        .find(|t| t["name"] == "mcp_smart-source__lookup")
        .ok_or("producer not discovered")?["name"]
        .clone();
    let sink_name = consumers["structuredContent"]["tools"]
        .as_array()
        .ok_or("missing consumer results")?
        .iter()
        .find(|t| t["name"] == "mcp_smart-sink__fetch")
        .ok_or("consumer not discovered")?["name"]
        .clone();
    let definition = json!({"steps":[
        {"id":"search","tool":source_name,"arguments":{"query":"{{inputs.query}}"}},
        {"id":"context","tool":"recall","arguments":{"repo":"global"}},
        {"id":"fetch","tool":sink_name,"for_each":"{{steps.search.json.results}}","when":"{{item.enabled}}","arguments":{"url":"{{item.url}}","sequence":"{{index}}"}},
        {"id":"optional","tool":"mcp_unavailable__read","on_error":"continue"}
    ],"output":{"items":"{{steps.fetch.json}}","optional_ok":"{{steps.optional.ok}}"}});
    let dry = rpc(
        app,
        token,
        "run_workflow",
        json!({"definition":definition,"dry_run":true,"inputs":{"query":"example"}}),
    )
    .await?;
    assert_eq!(dry["structuredContent"]["valid"], true, "{dry}");
    let worker_app = app.clone();
    let source_id = source["structuredContent"]["id"].clone();
    let sink_id = sink["structuredContent"]["id"].clone();
    let worker = tokio::spawn(async move {
        let source = rpc(
            &worker_app,
            token,
            "poll_mcp",
            json!({"connection_id":source_id,"wait_seconds":5}),
        )
        .await
        .map_err(|e| e.to_string())?;
        let job = &source["structuredContent"]["job"];
        assert_eq!(job["request"]["arguments"]["query"], "example");
        let records = json!({"results":[{"url":"https://example.com/a","enabled":true},{"url":"https://example.com/skipped","enabled":false},{"url":"https://example.com/b","enabled":true}]});
        rpc(&worker_app,token,"complete_mcp",json!({"connection_id":source_id,"job_id":job["id"],"lease":job["lease"],"result":{"content":[{"type":"text","text":records.to_string()}]}})).await.map_err(|e|e.to_string())?;
        for _ in 0..2 {
            let claimed = rpc(
                &worker_app,
                token,
                "poll_mcp",
                json!({"connection_id":sink_id,"wait_seconds":5}),
            )
            .await
            .map_err(|e| e.to_string())?;
            let job = &claimed["structuredContent"]["job"];
            let args = &job["request"]["arguments"];
            assert!(
                args["url"]
                    .as_str()
                    .is_some_and(|u| u == "https://example.com/a" || u == "https://example.com/b")
            );
            assert!(args["sequence"].is_number());
            rpc(&worker_app,token,"complete_mcp",json!({"connection_id":sink_id,"job_id":job["id"],"lease":job["lease"],"result":{"content":[{"type":"text","text":args.to_string()}],"structuredContent":args}})).await.map_err(|e|e.to_string())?;
        }
        Ok::<(), String>(())
    });
    let run = rpc(
        app,
        token,
        "run_workflow",
        json!({"definition":definition,"inputs":{"query":"example"},"wait_seconds":5}),
    )
    .await?;
    worker.await?.map_err(|e| format!("dynamic worker: {e}"))?;
    let r = &run["structuredContent"];
    assert_eq!(r["status"], "completed", "{r}");
    assert_eq!(r["output"]["items"][0]["url"], "https://example.com/a");
    assert!(r["output"]["items"][1].is_null());
    assert_eq!(r["output"]["items"][2]["sequence"], 2);
    assert_eq!(r["output"]["optional_ok"], false);
    assert_eq!(r["tool_calls"], 5);
    Ok(())
}
