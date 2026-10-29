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
use sea_orm::FromQueryResult;
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
    let values=Capability::find_by_statement(sea_orm::Statement::from_sql_and_values(sea_orm::DbBackend::Postgres, "SELECT * FROM capabilities WHERE active AND (strpos(lower(name),lower($1))>0 OR strpos(lower(description),lower($1))>0) ORDER BY created_at DESC,id LIMIT 50", vec![(query).into()])).all(&state.postgres).await?;
    Ok(Json(values))
}
