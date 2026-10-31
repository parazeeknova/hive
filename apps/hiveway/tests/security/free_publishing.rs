use super::{Result, attack};
use axum::{Router, http::StatusCode};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::json;

pub(super) async fn exercise(
    app: Router,
    bearer: &str,
    state: &hiveway::state::AppState,
) -> Result {
    scaffold_catalog(&app, bearer).await?;
    connector_roundtrip(&app, bearer, state).await?;
    let headers = [("authorization", bearer)];
    let (status, invalid) = attack(app.clone(), "POST", "/mcp", json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"scaffold_project","arguments":{"project_name":"new-app","repository_url":"http://127.0.0.1/private","revision":"a".repeat(40)}}}).to_string(), &headers).await?;
    assert_eq!(status, StatusCode::OK, "{invalid}");
    assert_eq!(invalid["result"]["isError"], true);
    let wasm = wat::parse_str(include_str!(
        "../../../../packages/local/fixtures/wasm/echo.wat"
    ))?;
    let schema = json!({"type":"object"});
    let (status, artifact) = attack(
        app.clone(),
        "POST",
        "/v1/artifacts",
        json!({"wasm_base64":STANDARD.encode(wasm),"input_schema":schema,"output_schema":schema})
            .to_string(),
        &headers,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{artifact}");
    let listing = json!({"name":"Free GitHub tool","description":"Published without a wallet","runtime":"wasm","artifact_id":artifact["id"],"price":"0"});
    let (status, capability) = attack(
        app.clone(),
        "POST",
        "/v1/capabilities",
        listing.to_string(),
        &headers,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{capability}");
    assert_eq!(capability["creator_wallet"], "github:123");
    let mut paid = listing;
    paid["price"] = json!("1");
    let (status, error) = attack(
        app.clone(),
        "POST",
        "/v1/capabilities",
        paid.to_string(),
        &headers,
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{error}");
    let (status, leaf) = attack(app.clone(), "POST", "/v1/capabilities", json!({"name":"Free repository tool","description":"Free owned workflow step","runtime":"repository_stack","price":"0"}).to_string(), &headers).await?;
    assert_eq!(status, StatusCode::OK, "{leaf}");
    let mut workflow = json!({"name":"Free GitHub workflow","description":"No wallet required","steps":[leaf["id"]],"price":"0"});
    let (status, result) = attack(
        app.clone(),
        "POST",
        "/v1/workflows",
        workflow.to_string(),
        &headers,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    workflow["price"] = json!("1");
    let (status, error) =
        attack(app, "POST", "/v1/workflows", workflow.to_string(), &headers).await?;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{error}");
    Ok(())
}

async fn scaffold_catalog(app: &Router, bearer: &str) -> Result {
    let headers = [("authorization", bearer)];
    let (status,cap)=attack(app.clone(),"POST","/v1/capabilities",json!({"name":"Pinned starter artifact","description":"Catalog scaffold plan","runtime":"repository_scaffold","price":"0"}).to_string(),&headers).await?;
    assert_eq!(status, StatusCode::OK, "{cap}");
    let key = uuid::Uuid::new_v4().to_string();
    let headers = [("authorization", bearer), ("idempotency-key", key.as_str())];
    let (status,quote)=attack(app.clone(),"POST","/v1/quotes",json!({"capability_id":cap["id"],"input":{"project_name":"new-app","repository_url":"https://github.com/any/starter","revision":"a".repeat(40)}}).to_string(),&headers).await?;
    assert_eq!(status, StatusCode::CREATED, "{quote}");
    let (_, tools) = attack(
        app.clone(),
        "POST",
        "/mcp",
        json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
        &[("authorization", bearer)],
    )
    .await?;
    let id = cap["id"]
        .as_str()
        .ok_or("missing scaffold capability")?
        .parse::<uuid::Uuid>()?;
    let definition = tools["result"]["tools"]
        .as_array()
        .ok_or("missing tools")?
        .iter()
        .find(|tool| tool["name"] == format!("hive_{}", id.simple()))
        .ok_or("missing scaffold tool")?;
    let description = definition["description"]
        .as_str()
        .ok_or("missing description")?;
    for expected in [
        "Pinned starter artifact",
        "Catalog scaffold plan",
        "Price 0 atomic units",
        "untrusted data",
    ] {
        assert!(description.contains(expected), "{description}");
    }
    let properties = &definition["inputSchema"]["properties"]["input"]["properties"];
    assert_eq!(properties["project_name"]["maxLength"], 63);
    assert_eq!(properties["revision"]["pattern"], "^[a-fA-F0-9]{40}$");
    assert!(
        properties["repository_url"]["pattern"]
            .as_str()
            .is_some_and(|pattern| pattern.contains("github"))
    );
    Ok(())
}

async fn connector_rpc(
    app: Router,
    bearer: &str,
    name: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value> {
    let (_, result)=attack(app,"POST","/mcp",json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args}}).to_string(),&[("authorization",bearer)]).await?;
    Ok(result["result"].clone())
}
async fn connector_roundtrip(
    app: &Router,
    bearer: &str,
    state: &hiveway::state::AppState,
) -> Result {
    let registered = connector_rpc(
        app.clone(),
        bearer,
        "register_mcp_connection",
        json!({"name":"fixture","tools":[{"name":"read_file","inputSchema":{"type":"object"}}]}),
    )
    .await?;
    assert_eq!(registered["isError"], false, "{registered}");
    let id = registered["structuredContent"]["id"]
        .as_str()
        .ok_or("missing connector id")?
        .to_owned();
    let name = format!("external_{}_read_file", id.replace('-', ""));
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("authorization", bearer.parse()?);
    let mut foreign = hiveway::features::identity::authenticate(state, &headers, "").await?;
    foreign.wallet = "github:connector-foreign".into();
    assert_eq!(
        hiveway::features::connections::list(state, &foreign, true).await?,
        json!([])
    );
    assert!(
        hiveway::features::connections::invoke(state, &foreign, &name, json!({}))
            .await
            .is_err()
    );
    assert!(
        hiveway::features::connections::manage(
            state,
            &foreign,
            "poll_mcp",
            json!({"connection_id":id})
        )
        .await
        .is_err()
    );
    let worker_app = app.clone();
    let worker_bearer = bearer.to_owned();
    let worker_id = id.clone();
    let worker = async move {
        for _ in 0..40 {
            let polled = connector_rpc(
                worker_app.clone(),
                &worker_bearer,
                "poll_mcp",
                json!({"connection_id":worker_id}),
            )
            .await?;
            let job = &polled["structuredContent"]["job"];
            if !job.is_null() {
                assert_eq!(job["request"]["name"], "read_file");
                let args = json!({"connection_id":worker_id,"job_id":job["id"],"lease":job["lease"],"result":{"content":[{"type":"text","text":"fixture content"}]}});
                let done = connector_rpc(
                    worker_app.clone(),
                    &worker_bearer,
                    "complete_mcp",
                    args.clone(),
                )
                .await?;
                assert_eq!(done["isError"], false);
                let replay =
                    connector_rpc(worker_app, &worker_bearer, "complete_mcp", args).await?;
                assert_eq!(replay["isError"], true);
                return Ok::<(), Box<dyn std::error::Error>>(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        Err("worker found no job".into())
    };
    let (called, completed) = tokio::join!(
        connector_rpc(app.clone(), bearer, &name, json!({"path":"example.txt"})),
        worker
    );
    completed?;
    let called = called?;
    assert_eq!(called["isError"], false, "{called}");
    assert_eq!(
        called["structuredContent"]["content"][0]["text"],
        "fixture content"
    );
    let deleted = connector_rpc(
        app.clone(),
        bearer,
        "disconnect_mcp",
        json!({"connection_id":id}),
    )
    .await?;
    assert_eq!(deleted["isError"], false);
    let removed = connector_rpc(app.clone(), bearer, &name, json!({})).await?;
    assert_eq!(removed["isError"], true);
    Ok(())
}
