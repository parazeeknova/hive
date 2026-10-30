use crate::entities::capabilities::{Column, Entity};
use crate::{
    features::{
        common::{
            error::{ApiError, ApiResult},
            model::{Capability, now},
        },
        identity,
    },
    state::AppState,
};
use axum::{Json, extract::State, http::HeaderMap};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ComposeRequest {
    pub name: String,
    pub description: String,
    pub steps: Vec<Uuid>,
    pub price: String,
}
#[utoipa::path(post,path="/v1/workflows",request_body=ComposeRequest,responses((status=200,body=Capability)))]
/// # Errors
/// Rejects invalid credentials, malformed workflows, unowned steps, and database failures.
pub async fn compose(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ComposeRequest>,
) -> ApiResult<Json<Capability>> {
    let creator = identity::authenticate(&state, &headers, "workflows:publish").await?;
    let price = identity::publishing_price(&creator.wallet, &request.price)?;
    if request.name.trim().is_empty()
        || request.name.len() > 80
        || request.description.trim().is_empty()
        || request.description.len() > 1000
        || request.steps.is_empty()
        || request.steps.len() > 4
    {
        return Err(ApiError::bad("invalid_workflow"));
    }
    let mut unique = request.steps.clone();
    unique.sort();
    unique.dedup();
    if unique.len() != request.steps.len() {
        return Err(ApiError::bad("duplicate_workflow_step"));
    }
    let tx = state.postgres.begin().await?;
    let steps = Entity::find()
        .filter(Column::Id.is_in(request.steps.clone()))
        .filter(Column::Active.eq(true))
        .filter(Column::CreatorWallet.eq(&creator.wallet))
        .filter(Column::Runtime.is_in(["repository_stack", "dependency_inventory"]))
        .all(&tx)
        .await?;
    if steps.len() != request.steps.len() {
        return Err(ApiError::bad(
            "workflow_steps_must_be_active_owned_leaf_capabilities",
        ));
    }
    let capability = crate::entities::capabilities::ActiveModel {
        id: Set(Uuid::new_v4()),
        creator_wallet: Set(creator.wallet),
        name: Set(request.name),
        description: Set(request.description),
        version: Set("1".into()),
        runtime: Set("workflow".into()),
        price: Set(price),
        active: Set(true),
        created_at: Set(now()),
        definition: Set(serde_json::json!({"steps":request.steps})),
    }
    .insert(&tx)
    .await?;

    tx.commit().await?;
    crate::features::scout::discovery::bump_catalog_version(&state).await;
    Ok(Json(capability.into()))
}
