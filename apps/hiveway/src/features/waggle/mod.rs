pub mod activity;
pub use activity::get as activity;
pub mod earnings;
use crate::entities::creator_sales::Column as CreatorSalesColumn;
use crate::entities::creator_sales::Entity as CreatorSalesEntity;
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
