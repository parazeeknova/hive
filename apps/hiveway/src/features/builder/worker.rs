use super::{BuildRequest, generation};
use crate::{
    entities::{
        agent_credentials,
        build_jobs::{self, Column, Entity},
        capabilities,
    },
    features::{
        common::{
            error::{ApiError, ApiResult},
            model::{Credential, now},
        },
        identity, wax,
    },
    state::AppState,
};
use sea_orm::sea_query::{Expr, LockType};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter,
    QuerySelect, Set, Statement, TransactionTrait,
};
use serde_json::json;

async fn authorize<C: ConnectionTrait>(db: &C, job: &build_jobs::Model) -> ApiResult<()> {
    let credential = agent_credentials::Entity::find_by_id(job.credential_id)
        .into_model::<Credential>()
        .one(db)
        .await?
        .ok_or_else(ApiError::forbidden)?;
    identity::authorize(&credential, "")?;
    if credential.wallet != job.owner {
        return Err(ApiError::forbidden());
    }
    Ok(())
}
/// Process one durable build at a time per worker, claiming safely across replicas.
/// # Errors
/// Reports database failures. Candidate/provider failures are persisted as failed jobs.
pub async fn tick(state: &AppState) -> ApiResult<()> {
    let Some(config) = &state.settings.builder else {
        return Ok(());
    };
    Entity::update_many()
        .col_expr(Column::State, Expr::value("failed"))
        .col_expr(Column::ErrorCode, Expr::value("builder_interrupted"))
        .col_expr(Column::UpdatedAt, Expr::value(now()))
        .filter(Column::State.eq("generating"))
        .filter(Column::UpdatedAt.lt(now() - 600))
        .exec(&state.postgres)
        .await?;
    let row=Entity::find().from_raw_sql(Statement::from_string(DbBackend::Postgres,"UPDATE build_jobs SET state='generating', updated_at=EXTRACT(EPOCH FROM NOW())::bigint WHERE id=(SELECT id FROM build_jobs WHERE state='queued' ORDER BY created_at,id FOR UPDATE SKIP LOCKED LIMIT 1) RETURNING *".to_owned())).one(&state.postgres).await?;
    let Some(mut job) = row else {
        return Ok(());
    };
    let outcome = if job.model == config.model {
        process(state, &mut job).await
    } else {
        Err(ApiError::bad("builder_model_changed"))
    };
    if let Err(error) = outcome {
        // A stale or already-published job must never be overwritten.
        Entity::update_many()
            .col_expr(Column::State, Expr::value("failed"))
            .col_expr(Column::ErrorCode, Expr::value(error.code))
            .col_expr(Column::UpdatedAt, Expr::value(now()))
            .filter(Column::Id.eq(job.id))
            .filter(Column::State.eq("generating"))
            .exec(&state.postgres)
            .await?;
    }
    Ok(())
}
async fn process(state: &AppState, job: &mut build_jobs::Model) -> ApiResult<()> {
    authorize(&state.postgres, job).await?;
    let request: BuildRequest = serde_json::from_value(job.request.clone())?;
    let mut previous: Option<(String, String)> = None;
    for attempt in 1..=2 {
        authorize(&state.postgres, job).await?;
        Entity::update_many()
            .col_expr(Column::Attempts, Expr::value(attempt))
            .col_expr(Column::UpdatedAt, Expr::value(now()))
            .filter(Column::Id.eq(job.id))
            .filter(Column::State.eq("generating"))
            .exec(&state.postgres)
            .await?;
        let generated = generation::generate(
            state,
            &request,
            previous
                .as_ref()
                .map(|(source, error)| (source.as_str(), error.as_str())),
        )
        .await?;
        let tested = generated.validate(state, &request).await;
        let report = match &tested {
            Ok((_, report)) | Err(report) => report.clone(),
        };
        Entity::update_many()
            .col_expr(Column::Source, Expr::value(generated.wat.clone()))
            .col_expr(Column::Validation, Expr::value(report.clone()))
            .col_expr(Column::UpdatedAt, Expr::value(now()))
            .filter(Column::Id.eq(job.id))
            .filter(Column::State.eq("generating"))
            .exec(&state.postgres)
            .await?;
        if let Ok((bytes, reports)) = tested
            && reports
                .as_array()
                .is_some_and(|rows| rows.iter().all(|r| r["passed"] == true))
        {
            publish(state, job, &request, &generated, bytes).await?;
            return Ok(());
        }
        previous = Some((
            serde_json::to_string(
                &json!({"wat":generated.wat,"input_schema":generated.input_schema,"output_schema":generated.output_schema}),
            )?,
            serde_json::to_string(&report)?,
        ));
    }
    Err(ApiError::bad("builder_validation_failed"))
}
async fn publish(
    state: &AppState,
    job: &build_jobs::Model,
    request: &BuildRequest,
    generated: &generation::Generated,
    bytes: Vec<u8>,
) -> ApiResult<()> {
    authorize(&state.postgres, job).await?;
    let artifact = wax::artifacts::repository::insert(
        state,
        &job.owner,
        bytes,
        generated.input_schema.clone(),
        generated.output_schema.clone(),
    )
    .await?;
    let tx = state.postgres.begin().await?;
    // Serialize final publication with stale-job recovery and credential revocation.
    let row = Entity::find_by_id(job.id)
        .lock(LockType::Update)
        .one(&tx)
        .await?
        .ok_or_else(ApiError::forbidden)?;
    if row.state != "generating" {
        return Err(ApiError::bad("builder_interrupted"));
    }
    let credential = agent_credentials::Entity::find_by_id(job.credential_id)
        .lock(LockType::Update)
        .one(&tx)
        .await?
        .ok_or_else(ApiError::forbidden)?;
    if credential.revoked_at.is_some()
        || credential.expires_at <= now()
        || credential.wallet != job.owner
    {
        return Err(ApiError::forbidden());
    }
    capabilities::ActiveModel {
        id: Set(job.id),
        creator_wallet: Set(job.owner.clone()),
        name: Set(request.name.clone()),
        description: Set(request.description.clone()),
        version: Set("1".into()),
        runtime: Set("wasm".into()),
        price: Set(0),
        active: Set(true),
        created_at: Set(now()),
        definition: Set(json!({"artifact_id":artifact.id,"abi_version":1})),
    }
    .insert(&tx)
    .await?;
    let mut model: build_jobs::ActiveModel = row.into();
    model.state = Set("published".into());
    model.artifact_id = Set(Some(artifact.id));
    model.capability_id = Set(Some(job.id));
    model.updated_at = Set(now());
    model.update(&tx).await?;
    tx.commit().await?;
    crate::features::scout::discovery::bump_catalog_version(state).await;
    Ok(())
}
