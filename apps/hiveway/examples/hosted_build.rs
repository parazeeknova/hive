//! Opt-in provider acceptance: uses `HIVE_BUILDER_*` and `HIVE_TEST_DATABASE_URL`.
//! Creates and removes an isolated database schema; makes up to two funded model calls.
use axum::{Json, extract::State, http::HeaderMap};
use hiveway::{
    entities::{agent_credentials, wasm_artifacts},
    features::{
        builder::{self, BuildRequest, Example},
        common::{model::now, settings::Settings},
        identity,
    },
    state::AppState,
};
use sea_orm::{ActiveModelTrait, ConnectionTrait, Database, EntityTrait, Set};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
#[tokio::main]
async fn main() -> Result {
    let url = std::env::var("HIVE_TEST_DATABASE_URL")?;
    let admin = Database::connect(url.clone()).await?;
    let schema = format!("hive_builder_acceptance_{}", Uuid::new_v4().simple());
    admin
        .execute_unprepared(&format!("CREATE SCHEMA {schema}"))
        .await?;
    let mut options = sea_orm::ConnectOptions::new(url);
    options
        .set_schema_search_path(schema.clone())
        .sqlx_logging(false);
    let db = Database::connect(options).await?;
    let result = exercise(db.clone()).await;
    db.close().await?;
    admin
        .execute_unprepared(&format!("DROP SCHEMA {schema} CASCADE"))
        .await?;
    admin.close().await?;
    result
}
async fn exercise(db: sea_orm::DatabaseConnection) -> Result {
    use sea_orm_migration::MigratorTrait;
    hiveway::migrations::Migrator::up(&db, None).await?;
    let settings = Settings::from_env()?;
    let model = settings
        .builder
        .as_ref()
        .ok_or("Configure HIVE_BUILDER_* for this opt-in example")?
        .model
        .clone();
    let state = AppState {
        sandbox: Arc::new(hiveway::features::cell::runtime::Sandbox::new(
            settings.runtime.sandbox.clone(),
        )),
        settings: Arc::new(settings),
        postgres: db,
        redis: fred::prelude::Builder::default_centralized().build()?,
        http: reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?,
    };
    let token = format!("hwa_{}", Uuid::new_v4().simple().to_string().repeat(2));
    agent_credentials::ActiveModel {
        id: Set(Uuid::new_v4()),
        wallet: Set("github:987654321".into()),
        name: Set("Acceptance".into()),
        token_hash: Set(identity::hash(token.as_bytes())),
        scopes: Set(vec!["capabilities:publish".into()]),
        per_call_limit: Set(1),
        total_limit: Set(1),
        spent: Set(0),
        reserved: Set(0),
        expires_at: Set(now() + 3600),
        revoked_at: Set(None),
        created_at: Set(now()),
    }
    .insert(&state.postgres)
    .await?;
    let mut headers = HeaderMap::new();
    headers.insert("authorization", format!("Bearer {token}").parse()?);
    let request=BuildRequest { idempotency_key:Uuid::new_v4(),name:"Double an integer".into(),description:"Doubles the n property of any input object".into(),task:"Accept a JSON object with required property n, an integer between 0 and 1000000 inclusive, no additional properties. Return an object with required property doubled containing n multiplied by 2. Handle all valid inputs generically, including zero and all decimal digit lengths.".into(),examples:vec![Example{input:json!({"n":3}),expected:json!({"doubled":6})},Example{input:json!({"n":7}),expected:json!({"doubled":14})},Example{input:json!({"n":0}),expected:json!({"doubled":0})}] };
    let started = std::time::Instant::now();
    let Json(job) = builder::submit(State(state.clone()), headers.clone(), Json(request)).await?;
    builder::worker::tick(&state).await?;
    let Json(job) =
        builder::get(State(state.clone()), axum::extract::Path(job.id), headers).await?;
    if job.state != "published" {
        return Err(format!(
            "Build {}: {:?}; validation {:?}",
            job.state, job.error_code, job.validation
        )
        .into());
    }
    let artifact = wasm_artifacts::Entity::find_by_id(job.artifact_id.ok_or("missing artifact")?)
        .one(&state.postgres)
        .await?
        .ok_or("missing artifact")?;
    for input in [1, 41, 72, 999_999, 1_000_000] {
        let output = state
            .sandbox
            .execute(
                artifact.wasm.clone(),
                json!({"n":input}),
                artifact.input_schema.clone(),
                artifact.output_schema.clone(),
            )
            .await?;
        assert_eq!(output, json!({"doubled":input*2}), "unseen input {input}");
    }
    println!(
        "Provider acceptance passed: model={model}, attempts={}, sample_tests=3, unseen_tests=5, elapsed_seconds={:.2}",
        job.attempts,
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
