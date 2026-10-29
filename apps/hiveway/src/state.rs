use std::{sync::Arc, time::Duration};

use anyhow::Context;
use fred::prelude::{Builder, ClientLike, Config as RedisConfig};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

use crate::config::Config;

/// Shared infrastructure handles. No capability or agent workflows are started.
#[derive(Clone)]
pub struct AppState {
    pub sandbox: Arc<crate::features::cell::runtime::Sandbox>,
    pub settings: Arc<crate::features::common::settings::Settings>,
    pub postgres: DatabaseConnection,
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
        let settings = Arc::new(crate::features::common::settings::Settings::from_env()?);
        let mut options = ConnectOptions::new(config.database_url.clone());
        options
            .max_connections(settings.runtime.infrastructure.postgres_connections)
            .connect_timeout(Duration::from_secs(
                settings.runtime.infrastructure.connection_timeout_seconds,
            ))
            .acquire_timeout(Duration::from_secs(
                settings.runtime.infrastructure.connection_timeout_seconds,
            ))
            .sqlx_logging(false);
        let postgres = Database::connect(options)
            .await
            .context("connecting Hiveway to PostgreSQL")?;
        crate::database::migrate(&postgres)
            .await
            .context("applying Hiveway migrations")?;
        let redis = Builder::from_config(RedisConfig::from_url(&config.redis_url)?)
            .with_connection_config(|config| {
                config.connection_timeout =
                    Duration::from_secs(settings.runtime.infrastructure.connection_timeout_seconds);
            })
            .build()?;
        redis.init().await.context("connecting Hiveway to Redis")?;
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(
                settings.runtime.infrastructure.connection_timeout_seconds,
            ))
            .timeout(Duration::from_secs(
                settings.runtime.infrastructure.outbound_timeout_seconds,
            ))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("hiveway/", env!("CARGO_PKG_VERSION")))
            .build()?;
        let rig = config
            .openai_api_key
            .as_deref()
            .map(rig_core::providers::openai::OpenAI::new);
        let sandbox = Arc::new(crate::features::cell::runtime::Sandbox::new(
            settings.runtime.sandbox.clone(),
        ));
        Ok(Self {
            sandbox,
            settings,
            postgres,
            redis,
            http,
            rig,
        })
    }
}
