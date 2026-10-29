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
    let values = crate::entities::creator_sales::Entity::find()
        .filter(crate::entities::creator_sales::Column::CreatorWallet.eq(creator.wallet))
        .order_by_desc(crate::entities::creator_sales::Column::CreatedAt)
        .order_by_desc(crate::entities::creator_sales::Column::InvocationId)
        .limit(100)
        .into_model::<Sale>()
        .all(&state.postgres)
        .await?;
    Ok(Json(values))
}
