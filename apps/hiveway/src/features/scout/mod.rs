pub mod detail;
pub mod discovery;
use crate::entities::capabilities::Column as CapabilitiesColumn;
use crate::entities::capabilities::Entity as CapabilitiesEntity;
use crate::{
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
    extract::{Query, State},
    http::HeaderMap,
};
use sea_orm::sea_query::{Alias, Expr, ExprTrait, Func, Order};
use sea_orm::{EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::Deserialize;
#[derive(Deserialize, utoipa::IntoParams)]
pub struct Search {
    pub q: Option<String>,
}
#[utoipa::path(get,path="/v1/capabilities",params(Search),responses((status=200,body=Vec<Capability>)))]
/// # Errors
/// Rejects invalid credentials, oversized searches, and database failures.
pub async fn discover(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(search): Query<Search>,
) -> ApiResult<Json<Vec<Capability>>> {
    identity::authenticate(&state, &headers, "capabilities:read").await?;
    let query = search.q.unwrap_or_default();
    if query.len() > 200 {
        return Err(ApiError::bad("search_too_long"));
    }
    let values = CapabilitiesEntity::find()
        .filter(Expr::col(CapabilitiesColumn::Active).and(matching(&query)))
        .order_by(Expr::col(CapabilitiesColumn::CreatedAt), Order::Desc)
        .order_by(Expr::col(CapabilitiesColumn::Id), Order::Asc)
        .limit(50_u64)
        .into_model::<Capability>()
        .all(&state.postgres)
        .await?;
    Ok(Json(values))
}

fn matching(query: &str) -> sea_orm::sea_query::SimpleExpr {
    Expr::expr(Func::cust(Alias::new("strpos")).args([
        Expr::expr(Func::lower(Expr::col(CapabilitiesColumn::Name))),
        Expr::expr(Func::lower(Expr::val(query))),
    ]))
    .gt(0_i64)
    .or(Expr::expr(Func::cust(Alias::new("strpos")).args([
        Expr::expr(Func::lower(Expr::col(CapabilitiesColumn::Description))),
        Expr::expr(Func::lower(Expr::val(query))),
    ]))
    .gt(0_i64))
}
