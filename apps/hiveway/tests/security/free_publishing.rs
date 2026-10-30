use super::{Result, attack};
use axum::{Router, http::StatusCode};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::json;

pub(super) async fn exercise(app: Router, bearer: &str) -> Result {
    let headers = [("authorization", bearer)];
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
