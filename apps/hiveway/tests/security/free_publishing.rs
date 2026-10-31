use super::{Result, attack};
use axum::{Router, http::StatusCode};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::json;

pub(super) async fn exercise(app: Router, bearer: &str) -> Result {
    scaffold_catalog(&app, bearer).await?;
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
