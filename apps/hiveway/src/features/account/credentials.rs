//! Account-managed free API keys. Secrets are returned once, stored only as hashes.
use crate::{
    entities::{account_api_keys as keys, agent_credentials as credentials},
    features::{
        common::{
            error::{ApiError, ApiResult},
            model::{Credential, now},
        },
        identity,
    },
    state::AppState,
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, QueryTrait, Set, TransactionTrait,
};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;
async fn manager(state: &AppState, headers: &HeaderMap) -> ApiResult<Credential> {
    let parent = identity::authenticate(state, headers, "capabilities:invoke").await?;
    if !parent.wallet.starts_with("github:")
        || keys::Entity::find_by_id(parent.id)
            .one(&state.postgres)
            .await?
            .is_some()
    {
        return Err(ApiError::forbidden());
    }
    Ok(parent)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateKey {
    #[serde(default)]
    name: Option<String>,
}
/// Create a named key restricted to free-use scopes already held by the browser session.
/// # Errors
/// Rejects invalid names, API-key management attempts, and database failures.
pub async fn issue(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateKey>,
) -> ApiResult<Json<Value>> {
    let parent = manager(&state, &headers).await?;
    let name = request
        .name
        .unwrap_or_else(|| "Personal agent".into())
        .trim()
        .to_owned();
    if name.is_empty() || name.chars().count() > 60 || name.chars().any(char::is_control) {
        return Err(ApiError::bad("invalid_api_key_name"));
    }
    super::ensure(&state, &parent).await?;
    let scopes: Vec<String> = parent
        .scopes
        .into_iter()
        .filter(|s| {
            [
                "capabilities:read",
                "capabilities:invoke",
                "repository:analyze",
                "receipts:read",
            ]
            .contains(&s.as_str())
        })
        .collect();
    let token = format!("hwa_{}", hex::encode(rand::random::<[u8; 32]>()));
    let id = Uuid::new_v4();
    let expires_at = parent.expires_at.min(now() + 3600);
    let tx = state.postgres.begin().await?;
    crate::entities::account_profiles::Entity::find_by_id(parent.wallet.clone())
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| ApiError::unavailable("profile_not_found"))?;
    let owned = keys::Entity::find()
        .select_only()
        .column(keys::Column::Id)
        .filter(keys::Column::Owner.eq(parent.wallet.clone()))
        .into_query();
    let count = credentials::Entity::find()
        .filter(credentials::Column::Id.in_subquery(owned))
        .filter(credentials::Column::RevokedAt.is_null())
        .count(&tx)
        .await?;
    if count >= 100 {
        return Err(ApiError::bad("api_key_limit_remove_an_old_key"));
    }
    credentials::ActiveModel {
        id: Set(id),
        wallet: Set(parent.wallet.clone()),
        name: Set(name.clone()),
        token_hash: Set(identity::hash(token.as_bytes())),
        scopes: Set(scopes),
        per_call_limit: Set(1),
        total_limit: Set(1),
        expires_at: Set(expires_at),
        created_at: Set(now()),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
    keys::ActiveModel {
        id: Set(id),
        owner: Set(parent.wallet),
    }
    .insert(&tx)
    .await?;
    tx.commit().await?;
    Ok(Json(
        json!({"id":id,"name":name,"token":token,"expires_at":expires_at}),
    ))
}
/// List owned key metadata, excluding browser sessions and secret material.
/// # Errors
/// Rejects unauthorized managers and database failures.
pub async fn list(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let parent = manager(&state, &headers).await?;
    let owned = keys::Entity::find()
        .select_only()
        .column(keys::Column::Id)
        .filter(keys::Column::Owner.eq(parent.wallet))
        .into_query();
    let rows = credentials::Entity::find()
        .filter(credentials::Column::Id.in_subquery(owned))
        .filter(credentials::Column::RevokedAt.is_null())
        .order_by_desc(credentials::Column::CreatedAt)
        .order_by_desc(credentials::Column::Id)
        .limit(100)
        .all(&state.postgres)
        .await?;
    Ok(Json(
        json!({"keys":rows.into_iter().map(|row|json!({"id":row.id,"name":row.name,"scopes":row.scopes,"created_at":row.created_at,"expires_at":row.expires_at,"expired":row.expires_at<=now()})).collect::<Vec<_>>()}),
    ))
}
/// Revoke an owned API key immediately without affecting browser login sessions.
/// # Errors
/// Rejects foreign or browser-session IDs and database failures.
pub async fn revoke(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    let parent = manager(&state, &headers).await?;
    let tx = state.postgres.begin().await?;
    let owned = keys::Entity::find_by_id(id)
        .filter(keys::Column::Owner.eq(parent.wallet))
        .one(&tx)
        .await?;
    if owned.is_none() {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "api_key_not_found"));
    }
    credentials::Entity::update_many()
        .col_expr(
            credentials::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(now()),
        )
        .filter(credentials::Column::Id.eq(id))
        .exec(&tx)
        .await?;
    tx.commit().await?;
    Ok(Json(json!({"revoked":true})))
}
