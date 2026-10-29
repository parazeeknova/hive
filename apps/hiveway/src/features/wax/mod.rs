pub mod artifacts;
use crate::{
    features::{
        common::{
            error::{ApiError, ApiResult},
            model::{Capability, PublishCapability, now},
        },
        identity,
    },
    state::AppState,
};
use axum::{Json, extract::State, http::HeaderMap};
use sea_orm::{ActiveModelTrait, Set};
use uuid::Uuid;

#[utoipa::path(post,path="/v1/capabilities",request_body=PublishCapability,responses((status=200,body=Capability)))]
/// # Errors
/// Rejects invalid credentials, unsupported runtimes, invalid prices, and database failures.
pub async fn publish(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<PublishCapability>,
) -> ApiResult<Json<Capability>> {
    let owner = identity::authenticate(&state, &headers, "capabilities:publish").await?;
    if request.name.trim().is_empty()
        || request.name.len() > 80
        || request.description.trim().is_empty()
        || request.description.len() > 1000
        || !matches!(
            request.runtime.as_str(),
            "repository_stack" | "dependency_inventory" | "wasm"
        )
    {
        return Err(ApiError::bad("invalid_capability"));
    }
    let definition = if request.runtime == "wasm" {
        let artifact_id = request
            .artifact_id
            .ok_or_else(|| ApiError::bad("artifact_id_required"))?;
        artifacts::owned(&state, artifact_id, &owner.wallet).await?;
        serde_json::json!({"artifact_id":artifact_id,"abi_version":1})
    } else {
        if request.artifact_id.is_some() {
            return Err(ApiError::bad("artifact_not_supported_for_builtin"));
        }
        serde_json::json!({})
    };
    let capability = crate::entities::capabilities::ActiveModel {
        id: Set(Uuid::new_v4()),
        creator_wallet: Set(owner.wallet),
        name: Set(request.name),
        description: Set(request.description),
        version: Set("1".into()),
        runtime: Set(request.runtime),
        price: Set(identity::amount(&request.price)?),
        active: Set(true),
        created_at: Set(now()),
        definition: Set(definition),
    }
    .insert(&state.postgres)
    .await?;

    Ok(Json(capability.into()))
}
