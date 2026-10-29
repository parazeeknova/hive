use crate::{
    entities::capabilities::Entity,
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
use sea_orm::EntityTrait;
use uuid::Uuid;

/// Read a pinned capability, including an inactive listing referenced by a receipt.
/// # Errors
/// Rejects invalid credentials and missing capability IDs.
#[utoipa::path(get,path="/v1/capabilities/{id}",params(("id"=Uuid,Path)),responses((status=200,body=Capability)))]
pub async fn detail(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Capability>> {
    identity::authenticate(&state, &headers, "capabilities:read").await?;
    let cap = Entity::find_by_id(id)
        .one(&state.postgres)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "capability_not_found"))?;
    Ok(Json(cap.into()))
}
