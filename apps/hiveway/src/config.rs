use std::env;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("required environment variable {0} is missing or empty")]
    Missing(&'static str),
    #[error("HIVE_BIND_ADDRESS must be a socket address")]
    InvalidBindAddress,
    #[error("invalid Hiveway operational configuration: {0:#}")]
    InvalidRuntimeConfiguration(#[source] anyhow::Error),
}

pub struct Config {
    pub bind_address: std::net::SocketAddr,
    pub database_url: String,
    pub redis_url: String,
}

impl Config {
    /// Load Hiveway's infrastructure configuration without exposing credentials.
    ///
    /// # Errors
    /// Returns an error when a required connection URL is absent.
    pub fn from_env() -> Result<Self, ConfigError> {
        let runtime = crate::runtime::RuntimeConfig::load()
            .map_err(ConfigError::InvalidRuntimeConfiguration)?;
        let required = |name| {
            env::var(name)
                .ok()
                .filter(|value| !value.is_empty())
                .ok_or(ConfigError::Missing(name))
        };
        Ok(Self {
            bind_address: env::var("HIVE_BIND_ADDRESS")
                .unwrap_or_else(|_| runtime.listen_address.to_string())
                .parse()
                .map_err(|_| ConfigError::InvalidBindAddress)?,
            database_url: required("DATABASE_URL")?,
            redis_url: required("REDIS_URL")?,
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn runtime_error_preserves_context_and_source() {
        use std::error::Error;
        let error = super::ConfigError::InvalidRuntimeConfiguration(
            anyhow::anyhow!("bad resource limit").context("validating configuration"),
        );
        assert!(error.to_string().contains("bad resource limit"));
        assert!(error.to_string().contains("validating configuration"));
        assert!(error.source().is_some());
    }
}
