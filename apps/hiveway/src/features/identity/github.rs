//! GitHub identities use a stable provider ID, never an invented payment address.
use super::{authenticate, hash};
use crate::{
    entities::{agent_credentials, capabilities},
    features::{
        cell,
        common::{
            error::{ApiError, ApiResult},
            model::now,
        },
    },
    state::AppState,
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exchange {
    code: String,
    verifier: String,
}
#[derive(Deserialize)]
struct GithubUser {
    id: u64,
    login: String,
}

/// Exchange a one-use OAuth code using PKCE and create a revocable database session.
/// # Errors
/// Rejects unconfigured OAuth, invalid codes and unavailable providers.
pub async fn exchange(
    State(state): State<AppState>,
    Json(input): Json<Exchange>,
) -> ApiResult<Json<Value>> {
    if input.code.len() > 256 || !(43..=128).contains(&input.verifier.len()) {
        return Err(ApiError::bad("invalid_oauth_request"));
    }
    let env = |key| {
        std::env::var(key)
            .ok()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| ApiError::unavailable("github_oauth_not_configured"))
    };
    let params = [
        ("client_id", env("GITHUB_CLIENT_ID")?),
        ("client_secret", env("GITHUB_CLIENT_SECRET")?),
        (
            "redirect_uri",
            format!("{}/api/auth/callback", state.settings.auth_uri),
        ),
        ("code", input.code),
        ("code_verifier", input.verifier),
    ];
    let mut url = reqwest::Url::parse("https://github.com/login/oauth/access_token")
        .map_err(|_| ApiError::bad("invalid_oauth_url"))?;
    // Form encoding without placing credentials in the URL or logs.
    url.query_pairs_mut()
        .extend_pairs(params.iter().map(|(k, v)| (*k, v.as_str())));
    let body = url.query().unwrap_or_default().to_owned();
    let response: Value = state
        .http
        .post("https://github.com/login/oauth/access_token")
        .header("accept", "application/json")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|_| ApiError::unavailable("github_unavailable"))?
        .json()
        .await
        .map_err(|_| ApiError::unavailable("github_invalid_response"))?;
    let access = response
        .get("access_token")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "github_login_failed"))?;
    let user: GithubUser = state
        .http
        .get("https://api.github.com/user")
        .bearer_auth(access)
        .send()
        .await
        .map_err(|_| ApiError::unavailable("github_unavailable"))?
        .error_for_status()
        .map_err(|_| ApiError::new(StatusCode::UNAUTHORIZED, "github_login_failed"))?
        .json()
        .await
        .map_err(|_| ApiError::unavailable("github_invalid_response"))?;
    let token = format!("hwa_{}", hex::encode(rand::random::<[u8; 32]>()));
    let expires = now() + 86400;
    agent_credentials::ActiveModel {
        id: Set(Uuid::new_v4()),
        wallet: Set(format!("github:{}", user.id)),
        name: Set(user.login),
        token_hash: Set(hash(token.as_bytes())),
        scopes: Set(vec![
            "capabilities:read".into(),
            "capabilities:invoke".into(),
            "repository:analyze".into(),
            "receipts:read".into(),
        ]),
        per_call_limit: Set(1_000_000),
        total_limit: Set(10_000_000),
        expires_at: Set(expires),
        created_at: Set(now()),
        ..Default::default()
    }
    .insert(&state.postgres)
    .await?;
    Ok(Json(json!({"token":token,"expires_at":expires})))
}
/// Revoke the current session in persistent storage.
/// # Errors
/// Rejects invalid sessions or database failures.
pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let c = authenticate(&state, &headers, "").await?;
    agent_credentials::Entity::update_many()
        .col_expr(
            agent_credentials::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(now()),
        )
        .filter(agent_credentials::Column::Id.eq(c.id))
        .exec(&state.postgres)
        .await?;
    Ok(Json(json!({"revoked":true})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestInput {
    input: Value,
}
/// Execute real input within existing sandbox limits. Paid previews are creator-only.
/// # Errors
/// Rejects unauthorized, inactive, invalid, or failing capabilities.
pub async fn test(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<TestInput>,
) -> ApiResult<Json<Value>> {
    let c = authenticate(&state, &headers, "capabilities:invoke").await?;
    let cap = capabilities::Entity::find_by_id(id)
        .one(&state.postgres)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "capability_not_found"))?;
    if !cap.active || (cap.price > 0 && cap.creator_wallet != c.wallet) {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "paid_sample_requires_creator_or_paid_invocation",
        ));
    }
    cell::validate_input(&state, id, &request.input).await?;
    let result = cell::execute_capability(&state, id, &request.input).await?;
    Ok(Json(json!({"capability_id":id,"result":result})))
}
