pub mod earnings;
use crate::entities::creator_sales::Column as CreatorSalesColumn;
use crate::entities::creator_sales::Entity as CreatorSalesEntity;
use crate::entities::invocations::Column as InvocationsColumn;
use crate::entities::invocations::Entity as InvocationsEntity;
use crate::{
    features::{
        common::{error::ApiResult, model::Sale},
        identity,
    },
    state::AppState,
};
use axum::{Json, extract::State, http::HeaderMap};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
#[utoipa::path(get,path="/v1/sales",responses((status=200,body=Vec<Sale>)))]
/// # Errors
/// Rejects invalid credentials and propagates database failures.
pub async fn sales(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<Sale>>> {
    let creator = identity::authenticate(&state, &headers, "sales:read").await?;
    let values = CreatorSalesEntity::find()
        .filter(CreatorSalesColumn::CreatorWallet.eq(creator.wallet))
        .order_by_desc(CreatorSalesColumn::CreatedAt)
        .order_by_desc(CreatorSalesColumn::InvocationId)
        .limit(100)
        .into_model::<Sale>()
        .all(&state.postgres)
        .await?;
    Ok(Json(values))
}

/// Read this credential's invocation history without exposing payment signatures.
/// # Errors
/// Rejects invalid credentials and returns database failures.
#[utoipa::path(get,path="/v1/activity",responses((status=200,body=serde_json::Value)))]
pub async fn activity(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let agent = identity::authenticate(&state, &headers, "receipts:read").await?;
    let records = InvocationsEntity::find()
        .filter(InvocationsColumn::CredentialId.eq(agent.id))
        .order_by_desc(InvocationsColumn::CreatedAt)
        .limit(100)
        .all(&state.postgres)
        .await?;
    Ok(Json(
        serde_json::json!({"invocations":records.into_iter().map(|r|serde_json::json!({"id":r.id,"capability_id":r.capability_id,"state":r.state,"price":r.price.to_string(),"payment_method":r.payment_method,"payment_issue":r.payment_issue,"execution_ms":r.execution_ms,"result_bytes":r.result_bytes,"created_at":r.created_at,"completed_at":r.completed_at})).collect::<Vec<_>>(),"budget":{"limit":agent.total_limit.to_string(),"spent":agent.spent.to_string(),"reserved":agent.reserved.to_string()}}),
    ))
}
