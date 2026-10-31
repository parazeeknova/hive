//! Batched public publisher identity and aggregate usage, never private invocation data.
use crate::{
    entities::{account_profiles as profiles, capabilities as caps, invocations as runs},
    features::{
        common::error::{ApiError, ApiResult},
        identity,
    },
    state::AppState,
};
use axum::{Json, extract::State, http::HeaderMap};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MetadataRequest {
    pub ids: Vec<Uuid>,
}
#[derive(Serialize, utoipa::ToSchema)]
pub struct Publisher {
    pub name: Option<String>,
    pub username: Option<String>,
    pub avatar: Option<String>,
    pub verified: bool,
    pub total_tools: i64,
}
#[derive(Serialize, utoipa::ToSchema)]
pub struct Metadata {
    pub capability_id: Uuid,
    pub invocation_count: i64,
    pub publisher: Publisher,
}
/// Load publisher profiles and aggregate counts for up to 100 capability IDs.
/// # Errors
/// Rejects invalid credentials, oversized batches and database failures.
#[utoipa::path(post,path="/v1/catalog/metadata",request_body=MetadataRequest,responses((status=200,body=Vec<Metadata>)))]
pub async fn get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<MetadataRequest>,
) -> ApiResult<Json<Vec<Metadata>>> {
    identity::authenticate(&state, &headers, "capabilities:read").await?;
    if request.ids.len() > 100 {
        return Err(ApiError::bad("metadata_batch_too_large"));
    }
    if request.ids.is_empty() {
        return Ok(Json(Vec::new()));
    }
    let capabilities = caps::Entity::find()
        .filter(caps::Column::Id.is_in(request.ids.clone()))
        .all(&state.postgres)
        .await?;
    let owners: Vec<String> = capabilities
        .iter()
        .map(|cap| cap.creator_wallet.clone())
        .collect();
    let profiles: HashMap<_, _> = profiles::Entity::find()
        .filter(profiles::Column::Owner.is_in(owners.clone()))
        .all(&state.postgres)
        .await?
        .into_iter()
        .map(|profile| (profile.owner.clone(), profile))
        .collect();
    let totals: HashMap<String, i64> = caps::Entity::find()
        .select_only()
        .column(caps::Column::CreatorWallet)
        .column_as(caps::Column::Id.count(), "total")
        .filter(caps::Column::CreatorWallet.is_in(owners))
        .group_by(caps::Column::CreatorWallet)
        .into_tuple()
        .all(&state.postgres)
        .await?
        .into_iter()
        .collect();
    let counts: HashMap<Uuid, i64> = runs::Entity::find()
        .select_only()
        .column(runs::Column::CapabilityId)
        .column_as(runs::Column::Id.count(), "total")
        .filter(runs::Column::CapabilityId.is_in(request.ids))
        .filter(runs::Column::StartedAt.is_not_null())
        .group_by(runs::Column::CapabilityId)
        .into_tuple()
        .all(&state.postgres)
        .await?
        .into_iter()
        .collect();
    Ok(Json(
        capabilities
            .into_iter()
            .map(|cap| {
                let profile = profiles.get(&cap.creator_wallet);
                Metadata {
                    capability_id: cap.id,
                    invocation_count: *counts.get(&cap.id).unwrap_or(&0),
                    publisher: Publisher {
                        name: profile.map(|p| p.name.clone()),
                        username: profile.map(|p| p.username.clone()),
                        avatar: profile.map(|p| p.avatar.clone()),
                        verified: profile.is_some_and(|p| p.publisher_verified),
                        total_tools: *totals.get(&cap.creator_wallet).unwrap_or(&0),
                    },
                }
            })
            .collect(),
    ))
}
