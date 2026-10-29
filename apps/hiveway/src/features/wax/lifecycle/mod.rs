//! Availability changes listing/quote admission; immutable code, price and prior quotes stay pinned.
use crate::{
    entities::capabilities::{Column, Entity},
    features::{
        common::{
            error::{ApiError, ApiResult},
            model::Capability,
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
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Availability {
    pub active: bool,
}

/// Change availability for a capability owned by the credential's wallet.
/// # Errors
/// Rejects foreign capabilities, insufficient publishing scope and invalid credentials.
#[utoipa::path(post,path="/v1/capabilities/{id}/availability",params(("id"=Uuid,Path)),request_body=Availability,responses((status=200,body=Capability)))]
pub async fn availability(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<Availability>,
) -> ApiResult<Json<Capability>> {
    let owner = identity::authenticate(&state, &headers, "").await?;
    let tx = state.postgres.begin().await?;
    let cap = Entity::find_by_id(id)
        .filter(Column::CreatorWallet.eq(&owner.wallet))
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "capability_not_found"))?;
    identity::authorize(
        &owner,
        if cap.runtime == "workflow" {
            "workflows:publish"
        } else {
            "capabilities:publish"
        },
    )?;
    let mut cap: crate::entities::capabilities::ActiveModel = cap.into();
    cap.active = Set(request.active);
    let cap = cap.update(&tx).await?;
    tx.commit().await?;
    crate::features::scout::discovery::bump_catalog_version(&state).await;
    Ok(Json(cap.into()))
}
