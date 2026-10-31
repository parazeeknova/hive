//! Persistent, quota-limited generation of free, tested pure Wasm capabilities.
pub mod config;
mod generation;
mod stream;
pub mod worker;
use crate::{
    entities::build_jobs::{self, Column, Entity},
    features::{
        common::{
            error::{ApiError, ApiResult},
            model::now,
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
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Example {
    pub input: Value,
    pub expected: Value,
}
#[derive(Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BuildRequest {
    pub idempotency_key: Uuid,
    pub name: String,
    pub description: String,
    pub task: String,
    pub examples: Vec<Example>,
}
impl BuildRequest {
    fn validate(&self) -> ApiResult<()> {
        if self.name.trim().is_empty()
            || self.name.len() > 80
            || self.description.trim().is_empty()
            || self.description.len() > 1000
            || self.task.trim().is_empty()
            || self.task.len() > 4096
            || !(1..=8).contains(&self.examples.len())
            || serde_json::to_vec(self)?.len() > 16384
        {
            return Err(ApiError::bad("invalid_build_request"));
        }
        if self
            .examples
            .iter()
            .any(|example| !example.input.is_object() || !example.expected.is_object())
        {
            return Err(ApiError::bad("build_examples_require_objects"));
        }
        Ok(())
    }
}
#[derive(Serialize, utoipa::ToSchema)]
pub struct Build {
    pub id: Uuid,
    pub state: String,
    pub request: Value,
    pub model: String,
    pub attempts: i32,
    pub source: Option<String>,
    pub validation: Option<Value>,
    pub artifact_id: Option<Uuid>,
    pub capability_id: Option<Uuid>,
    pub error_code: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
impl From<build_jobs::Model> for Build {
    fn from(row: build_jobs::Model) -> Self {
        Self {
            id: row.id,
            state: row.state,
            request: row.request,
            model: row.model,
            attempts: row.attempts,
            source: row.source,
            validation: row.validation,
            artifact_id: row.artifact_id,
            capability_id: row.capability_id,
            error_code: row.error_code,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
/// Queue a hosted build. Idempotent retries do not consume another quota slot.
/// # Errors
/// Requires publishing permission, valid examples, provider configuration and remaining quotas.
#[utoipa::path(post,path="/v1/builds",request_body=BuildRequest,responses((status=200,body=Build)))]
pub async fn submit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<BuildRequest>,
) -> ApiResult<Json<Build>> {
    let owner = identity::authenticate(&state, &headers, "capabilities:publish").await?;
    request.validate()?;
    let config = state
        .settings
        .builder
        .as_ref()
        .ok_or_else(|| ApiError::unavailable("builder_not_configured"))?;
    let payload = serde_json::to_value(&request)?;
    let hash = identity::hash(&serde_json::to_vec(&payload)?);
    let tx = state.postgres.begin().await?;
    tx.query_one(
        &sea_orm::sea_query::Query::select()
            .expr(
                sea_orm::sea_query::Func::cust(sea_orm::sea_query::Alias::new(
                    "pg_advisory_xact_lock",
                ))
                .arg(7_158_324_861_i64),
            )
            .to_owned(),
    )
    .await?;
    let jobs = Entity::find().filter(Column::Owner.eq(&owner.wallet));
    if let Some(row) = jobs
        .clone()
        .filter(Column::IdempotencyKey.eq(request.idempotency_key))
        .one(&tx)
        .await?
    {
        if row.request_hash != hash {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "build_idempotency_conflict",
            ));
        }
        tx.commit().await?;
        return Ok(Json(row.into()));
    }
    if jobs
        .clone()
        .filter(Column::State.is_in(["queued", "generating"]))
        .count(&tx)
        .await?
        > 0
    {
        return Err(ApiError::new(StatusCode::CONFLICT, "build_already_active"));
    }
    let since = now() - 86400;
    if jobs.filter(Column::CreatedAt.gte(since)).count(&tx).await? >= config.owner_daily
        || Entity::find()
            .filter(Column::CreatedAt.gte(since))
            .count(&tx)
            .await?
            >= config.global_daily
    {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "build_daily_limit",
        ));
    }
    let row = build_jobs::ActiveModel {
        id: Set(Uuid::new_v4()),
        owner: Set(owner.wallet),
        credential_id: Set(owner.id),
        idempotency_key: Set(request.idempotency_key),
        request_hash: Set(hash),
        request: Set(payload),
        state: Set("queued".into()),
        model: Set(config.model.clone()),
        attempts: Set(0),
        source: Set(None),
        validation: Set(None),
        artifact_id: Set(None),
        capability_id: Set(None),
        error_code: Set(None),
        created_at: Set(now()),
        updated_at: Set(now()),
    }
    .insert(&tx)
    .await?;
    tx.commit().await?;
    Ok(Json(row.into()))
}
/// Read an owned build, including generated source and sample-test results.
/// # Errors
/// Rejects missing credentials and non-owned or unknown build IDs.
#[utoipa::path(get,path="/v1/builds/{id}",params(("id"=Uuid,Path)),responses((status=200,body=Build)))]
pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Build>> {
    let owner = identity::authenticate(&state, &headers, "capabilities:publish").await?;
    let row = Entity::find_by_id(id)
        .filter(Column::Owner.eq(owner.wallet))
        .one(&state.postgres)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "build_not_found"))?;
    Ok(Json(row.into()))
}
/// List the twenty latest owned builds, omitting generated source.
/// # Errors
/// Rejects missing credentials and database failures.
#[utoipa::path(get,path="/v1/builds",responses((status=200,body=Vec<Build>)))]
pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<Build>>> {
    let owner = identity::authenticate(&state, &headers, "capabilities:publish").await?;
    let rows = Entity::find()
        .filter(Column::Owner.eq(owner.wallet))
        .order_by_desc(Column::CreatedAt)
        .order_by_desc(Column::Id)
        .limit(20)
        .all(&state.postgres)
        .await?;
    Ok(Json(
        rows.into_iter()
            .map(|mut row| {
                row.source = None;
                row.into()
            })
            .collect(),
    ))
}

pub(crate) async fn call(
    state: AppState,
    headers: HeaderMap,
    name: &str,
    args: Value,
) -> ApiResult<Value> {
    let Json(value) = if name == "build_capability" {
        submit(
            State(state),
            headers,
            Json(
                serde_json::from_value(args)
                    .map_err(|_| ApiError::bad("invalid_build_arguments"))?,
            ),
        )
        .await?
    } else {
        let id = args["build_id"]
            .as_str()
            .and_then(|id| id.parse().ok())
            .ok_or_else(|| ApiError::bad("invalid_build_arguments"))?;
        get(State(state), Path(id), headers).await?
    };
    Ok(serde_json::to_value(value)?)
}
