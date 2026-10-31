//! Immutable, owned executable artifacts with bounded schemas and no host capabilities.
pub mod model;
pub(crate) mod repository;
use crate::{
    entities::wasm_artifacts::{Column, Entity},
    features::{
        common::error::{ApiError, ApiResult},
        identity,
    },
    state::AppState,
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use model::{Artifact, UploadArtifact};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

/// Validate and store an immutable Wasm artifact owned by this wallet.
/// # Errors
/// Rejects invalid credentials, oversized modules/schemas, sandbox violations and owner quota exhaustion.
#[utoipa::path(post,path="/v1/artifacts",request_body=UploadArtifact,responses((status=200,body=Artifact)))]
pub async fn upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<UploadArtifact>,
) -> ApiResult<Json<Artifact>> {
    let credential = identity::authenticate(&state, &headers, "capabilities:publish").await?;
    if request.wasm_base64.len() > state.settings.runtime.sandbox.artifact_bytes * 4 / 3 + 4 {
        return Err(ApiError::bad("wasm_artifact_too_large"));
    }
    let bytes = STANDARD
        .decode(&request.wasm_base64)
        .map_err(|_| ApiError::bad("invalid_wasm_encoding"))?;
    state
        .sandbox
        .verify(
            bytes.clone(),
            request.input_schema.clone(),
            request.output_schema.clone(),
        )
        .await?;
    Ok(Json(
        repository::insert(
            &state,
            &credential.wallet,
            bytes,
            request.input_schema,
            request.output_schema,
        )
        .await?
        .into(),
    ))
}
/// Read artifact metadata and schemas without returning executable bytes.
/// # Errors
/// Rejects missing credentials, unknown artifacts and database failures.
#[utoipa::path(get,path="/v1/artifacts/{id}",params(("id"=Uuid,Path)),responses((status=200,body=Artifact)))]
pub async fn metadata(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Artifact>> {
    identity::authenticate(&state, &headers, "capabilities:read").await?;
    let artifact = Entity::find_by_id(id)
        .one(&state.postgres)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "artifact_not_found"))?;
    Ok(Json(artifact.into()))
}
/// Load an artifact owned by the capability's publishing wallet.
/// # Errors
/// Rejects missing artifacts and cross-wallet references.
pub async fn owned(
    state: &AppState,
    id: Uuid,
    wallet: &str,
) -> ApiResult<crate::entities::wasm_artifacts::Model> {
    Entity::find_by_id(id)
        .filter(Column::OwnerWallet.eq(wallet))
        .one(&state.postgres)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "artifact_not_found"))
}
