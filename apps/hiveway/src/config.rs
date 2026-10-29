use std::env;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("required environment variable {0} is missing or empty")]
    Missing(&'static str),
    #[error("HIVE_BIND_ADDRESS must be a socket address")]
    InvalidBindAddress,
}

pub struct Config {
    pub bind_address: std::net::SocketAddr,
    pub database_url: String,
    pub redis_url: String,
    pub openai_api_key: Option<String>,
}

impl Config {
    /// Load Hiveway's infrastructure configuration without exposing credentials.
    ///
    /// # Errors
    /// Returns an error when a required connection URL is absent.
    pub fn from_env() -> Result<Self, ConfigError> {
        let required = |name| {
            env::var(name)
                .ok()
                .filter(|value| !value.is_empty())
                .ok_or(ConfigError::Missing(name))
        };
        Ok(Self {
            bind_address: env::var("HIVE_BIND_ADDRESS")
                .unwrap_or_else(|_| "127.0.0.1:3001".into())
                .parse()
                .map_err(|_| ConfigError::InvalidBindAddress)?,
            database_url: required("DATABASE_URL")?,
            redis_url: required("REDIS_URL")?,
            openai_api_key: env::var("OPENAI_API_KEY")
                .ok()
                .filter(|key| !key.is_empty()),
        })
    }
}
