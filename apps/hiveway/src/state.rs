use std::time::Duration;

use anyhow::Context;
use fred::prelude::{Builder, ClientLike, Config as RedisConfig};
use sqlx::postgres::PgPoolOptions;

use crate::config::Config;

/// Shared infrastructure handles. No capability or agent workflows are started.
#[derive(Clone)]
pub struct AppState {
    pub postgres: sqlx::PgPool,
    pub redis: fred::clients::Client,
    pub http: reqwest::Client,
    pub rig: Option<rig_core::providers::openai::OpenAI>,
}

impl AppState {
    /// Connect local infrastructure and prepare outbound clients.
    ///
    /// # Errors
    /// Returns an error when infrastructure is unavailable or a client cannot be built.
    pub async fn connect(config: &Config) -> anyhow::Result<Self> {
        let postgres = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .connect(&config.database_url)
            .await
            .context("connecting Hiveway to PostgreSQL")?;
        let redis = Builder::from_config(RedisConfig::from_url(&config.redis_url)?)
            .with_connection_config(|config| {
                config.connection_timeout = Duration::from_secs(5);
            })
            .build()?;
        redis.init().await.context("connecting Hiveway to Redis")?;
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("hiveway/", env!("CARGO_PKG_VERSION")))
            .build()?;
        let rig = config
            .openai_api_key
            .as_deref()
            .map(rig_core::providers::openai::OpenAI::new);
        Ok(Self {
            postgres,
            redis,
            http,
            rig,
        })
    }
}
