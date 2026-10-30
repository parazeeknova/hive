use super::{Result, attack};
use axum::{Router, http::StatusCode};
use hiveway::{
    entities::agent_credentials,
    features::{common::model::now, identity},
    state::AppState,
};
use sea_orm::{ActiveModelTrait, Set};
use serde_json::Value;
use uuid::Uuid;
pub(super) async fn exercise(
    app: Router,
    state: &AppState,
    headers: &[(&str, &str)],
    issued: &Value,
    bearer: &str,
) -> Result {
    let id = issued["id"].as_str().ok_or("missing key ID")?;
    let revoke = format!("/v1/account/credentials/{id}/revoke");
    let (status, list) = attack(
        app.clone(),
        "GET",
        "/v1/account/credentials",
        String::new(),
        headers,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let rows = list["keys"].as_array().ok_or("missing keys")?;
    assert!(rows.iter().any(|key| key["id"] == id));
    assert!(
        rows.iter()
            .all(|key| key.get("token").is_none() && key.get("token_hash").is_none())
    );
    let (_, browser) = attack(
        app.clone(),
        "GET",
        "/v1/credentials/me",
        String::new(),
        headers,
    )
    .await?;
    assert!(rows.iter().all(|key| key["id"] != browser["id"]));
    reject_key_management(app.clone(), bearer).await?;
    foreign_owner(app.clone(), state, &revoke).await?;
    let browser_id = browser["id"].as_str().ok_or("missing browser ID")?;
    let (status, _) = attack(
        app.clone(),
        "POST",
        &format!("/v1/account/credentials/{browser_id}/revoke"),
        "{}".into(),
        headers,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "key management cannot revoke browser sessions"
    );
    let (status, _) = attack(app.clone(), "POST", &revoke, "{}".into(), headers).await?;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = attack(
        app.clone(),
        "GET",
        "/v1/credentials/me",
        String::new(),
        &[("authorization", bearer)],
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "removed keys must stop authenticating"
    );
    let (status, list) = attack(
        app.clone(),
        "GET",
        "/v1/account/credentials",
        String::new(),
        headers,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert!(
        list["keys"]
            .as_array()
            .ok_or("missing keys")?
            .iter()
            .all(|key| key["id"] != id)
    );
    let (status, _) = attack(app, "GET", "/v1/credentials/me", String::new(), headers).await?;
    assert_eq!(status, StatusCode::OK, "browser login must remain usable");
    Ok(())
}
async fn foreign_owner(app: Router, state: &AppState, revoke: &str) -> Result {
    let token = format!("hwa_{}", hex::encode(rand::random::<[u8; 32]>()));
    agent_credentials::ActiveModel {
        id: Set(Uuid::new_v4()),
        wallet: Set("github:124".into()),
        name: Set("another-user".into()),
        token_hash: Set(identity::hash(token.as_bytes())),
        scopes: Set(vec!["capabilities:invoke".into()]),
        per_call_limit: Set(1),
        total_limit: Set(1),
        expires_at: Set(now() + 3600),
        created_at: Set(now()),
        ..Default::default()
    }
    .insert(&state.postgres)
    .await?;
    let bearer = format!("Bearer {token}");
    let headers = [("authorization", bearer.as_str())];
    let (status, list) = attack(
        app.clone(),
        "GET",
        "/v1/account/credentials",
        String::new(),
        &headers,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list["keys"].as_array().ok_or("missing keys")?.len(), 0);
    let (status, _) = attack(app, "POST", revoke, "{}".into(), &headers).await?;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "another account cannot revoke the key"
    );
    Ok(())
}

async fn reject_key_management(app: Router, bearer: &str) -> Result {
    let (status, _) = attack(
        app.clone(),
        "GET",
        "/v1/account/credentials",
        String::new(),
        &[("authorization", bearer)],
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "API keys must not manage keys"
    );
    let (status, _) = attack(
        app.clone(),
        "POST",
        "/v1/account/credentials",
        "{}".into(),
        &[("authorization", bearer)],
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "API keys cannot create more credentials"
    );
    let (status, _) = attack(
        app,
        "POST",
        "/v1/account",
        serde_json::json!({"name":"Changed","username":"changed-profile","avatar":""}).to_string(),
        &[("authorization", bearer)],
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "free API keys cannot edit account settings"
    );
    Ok(())
}
