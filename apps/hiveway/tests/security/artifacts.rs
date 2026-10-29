//! Adversarial publishing and dynamic-input contract checks against real PostgreSQL.
use super::{Result, attack};
use axum::{Router, http::StatusCode};
use base64::{Engine, engine::general_purpose::STANDARD};
use hiveway::{features::cell, state::AppState};
use serde_json::{Value, json};
use uuid::Uuid;

pub(super) async fn exercise(app: Router, state: &AppState, tokens: (&str, &str)) -> Result {
    let a = format!("Bearer {}", tokens.0);
    let b = format!("Bearer {}", tokens.1);
    let (status, public) = attack(app.clone(), "GET", "/api/config", String::new(), &[]).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        public["payment"]["asset"],
        state.settings.runtime.payment.asset
    );
    assert_eq!(public["features"]["wasm_artifacts"], true);
    assert!(public.get("dodo_api_key").is_none());
    assert!(!public.to_string().contains("fixture"));
    let bytes = wat::parse_str(include_str!(
        "../../../../packages/local/fixtures/wasm/echo.wat"
    ))?;
    let schema = json!({"type":"object","required":["value"],"properties":{"value":{"type":"string"}},"additionalProperties":false});
    let upload =
        json!({"wasm_base64":STANDARD.encode(&bytes),"input_schema":schema,"output_schema":schema});
    let (status, artifact) = post(app.clone(), "/v1/artifacts", &upload, &a).await?;
    assert_eq!(status, StatusCode::OK, "{artifact}");
    assert_eq!(artifact["byte_length"], bytes.len());
    assert!(artifact.get("wasm").is_none());
    let (status, replay) = post(app.clone(), "/v1/artifacts", &upload, &a).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        artifact["id"], replay["id"],
        "upload replay must deduplicate"
    );
    let definition = json!({"name":"Dynamic echo","description":"Pure JSON capability","runtime":"wasm","artifact_id":artifact["id"],"price":"1"});
    let (status, _) = post(app.clone(), "/v1/capabilities", &definition, &b).await?;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "cross-wallet artifact publication"
    );
    let (status, capability) = post(app.clone(), "/v1/capabilities", &definition, &a).await?;
    assert_eq!(status, StatusCode::OK, "{capability}");
    let id: Uuid = serde_json::from_value(capability["id"].clone())?;
    for value in ["first", "different request"] {
        let input = json!({"value":value});
        assert_eq!(cell::execute_capability(state, id, &input).await?, input);
    }
    assert!(
        cell::execute_capability(state, id, &json!({"value":42}))
            .await
            .is_err()
    );
    let quote = json!({"capability_id":id,"input":{"value":"paid dynamically"}});
    let key = Uuid::new_v4().to_string();
    let (status, _) = attack(
        app.clone(),
        "POST",
        "/v1/quotes",
        quote.to_string(),
        &[("authorization", &b), ("idempotency-key", &key)],
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "legacy repository credentials cannot run creator code"
    );
    let (status, _) = attack(
        app.clone(),
        "POST",
        "/v1/quotes",
        quote.to_string(),
        &[("authorization", &a), ("idempotency-key", &key)],
    )
    .await?;
    assert_eq!(status, StatusCode::CREATED);
    reject_uploads(app, &upload, &a).await
}
async fn post(app: Router, path: &str, body: &Value, bearer: &str) -> Result<(StatusCode, Value)> {
    attack(
        app,
        "POST",
        path,
        body.to_string(),
        &[("authorization", bearer)],
    )
    .await
}
async fn reject_uploads(app: Router, valid: &Value, bearer: &str) -> Result {
    let mut invalid = valid.clone();
    invalid["input_schema"]["$ref"] = json!("http://127.0.0.1/private");
    let (status, _) = post(app.clone(), "/v1/artifacts", &invalid, bearer).await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let mut invalid = valid.clone();
    invalid["wasm_base64"] = json!(STANDARD.encode(wat::parse_str(
        "(module (import \"env\" \"read_secret\" (func)))"
    )?));
    let (status, _) = post(app.clone(), "/v1/artifacts", &invalid, bearer).await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let mut invalid = valid.clone();
    invalid["owner_wallet"] = json!("0x2222222222222222222222222222222222222222");
    let (status, _) = post(app, "/v1/artifacts", &invalid, bearer).await?;
    assert!(status.is_client_error());
    Ok(())
}
