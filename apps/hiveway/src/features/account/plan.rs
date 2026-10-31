//! The current free tier has build credits, distinct from wallets and USDC budgets.
use crate::{
    entities::build_jobs::{Column, Entity},
    features::{
        common::{error::ApiResult, model::now},
        identity,
    },
    state::AppState,
};
use axum::{Json, extract::State, http::HeaderMap};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde_json::{Value, json};
/// Read server-backed daily build credits for the signed-in account.
/// # Errors
/// Rejects invalid credentials and database failures.
#[utoipa::path(get,path="/v1/account/plan",responses((status=200,body=Value)))]
pub async fn get(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let account = identity::authenticate(&state, &headers, "").await?;
    let Some(config) = &state.settings.builder else {
        return Ok(Json(json!({"tier":"free","build_credits":null})));
    };
    let daily = Entity::find().filter(Column::CreatedAt.gte(now() - 86400));
    let used = daily
        .clone()
        .filter(Column::Owner.eq(account.wallet))
        .count(&state.postgres)
        .await?;
    let global_used = daily.count(&state.postgres).await?;
    let available = config
        .owner_daily
        .saturating_sub(used)
        .min(config.global_daily.saturating_sub(global_used));
    Ok(Json(
        json!({"tier":"free","build_credits":{"available":available,"account_limit":config.owner_daily,"used":used,"window_hours":24}}),
    ))
}
