//! Validated, public operational configuration. Provider credentials stay in env.
use anyhow::{Context, bail};
use serde::Deserialize;
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfig {
    pub listen_address: std::net::SocketAddr,
    pub public_url: String,
    pub auth_origin: String,
    pub facilitator_url: String,
    pub rpc_url: String,
    pub dodo_api_url: String,
    pub http: HttpLimits,
    pub infrastructure: Infrastructure,
    pub worker: Worker,
    pub payment: Payment,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpLimits {
    pub concurrency: usize,
    pub requests_per_minute: u32,
    pub tracked_peers: usize,
    pub body_bytes: usize,
    pub body_timeout_seconds: u64,
    pub request_timeout_seconds: u64,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Infrastructure {
    pub postgres_connections: u32,
    pub connection_timeout_seconds: u64,
    pub outbound_timeout_seconds: u64,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Worker {
    pub interval_seconds: u64,
    pub challenge_retention_seconds: i64,
    pub batch_size: i64,
    pub webhook_max_attempts: u32,
    pub execution_timeout_seconds: u64,
    pub stale_after_seconds: i64,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Payment {
    pub chain_id: u64,
    pub asset: String,
    pub name: String,
    pub version: String,
}
impl Payment {
    #[must_use]
    pub fn network(&self) -> String {
        format!("eip155:{}", self.chain_id)
    }
}
impl RuntimeConfig {
    /// Load the configured file or the bundled public defaults.
    /// # Errors
    /// Rejects missing files, malformed values, and limits outside safety bounds.
    pub fn load() -> anyhow::Result<Self> {
        let text = std::env::var("HIVE_CONFIG_FILE")
            .map_or_else(
                |_| Ok(include_str!("../config.toml").to_owned()),
                std::fs::read_to_string,
            )
            .context("reading Hiveway operational configuration")?;
        let config: Self =
            toml::from_str(&text).context("parsing Hiveway operational configuration")?;
        config.validate()?;
        Ok(config)
    }
    /// Check operational values before accepting traffic.
    /// # Errors
    /// Rejects zero, excessive, or inconsistent limits and malformed payment assets.
    pub fn validate(&self) -> anyhow::Result<()> {
        let h = &self.http;
        let i = &self.infrastructure;
        let w = &self.worker;
        // These are security ceilings, not runtime tuning defaults.
        if !(1..=1024).contains(&h.concurrency)
            || !(1..=100_000).contains(&h.requests_per_minute)
            || !(1..=65_536).contains(&h.tracked_peers)
            || !(1024..=1_048_576).contains(&h.body_bytes)
            || !(1..=30).contains(&h.body_timeout_seconds)
            || !(1..=600).contains(&h.request_timeout_seconds)
            || !(1..=100).contains(&i.postgres_connections)
            || !(1..=60).contains(&i.connection_timeout_seconds)
            || !(1..=60).contains(&i.outbound_timeout_seconds)
            || w.challenge_retention_seconds <= 60
            || !(1..=60).contains(&w.interval_seconds)
            || !(1..=100).contains(&w.batch_size)
            || !(1..=1000).contains(&w.webhook_max_attempts)
            || !(1..=120).contains(&w.execution_timeout_seconds)
            || w.stale_after_seconds
                <= i64::try_from(w.execution_timeout_seconds + i.outbound_timeout_seconds * 2)
                    .unwrap_or(i64::MAX)
            || h.request_timeout_seconds
                < w.execution_timeout_seconds + i.outbound_timeout_seconds * 2
        {
            bail!("invalid or inconsistent Hiveway resource limits");
        }
        if self.payment.chain_id == 0
            || self.payment.name.is_empty()
            || self.payment.name.len() > 80
            || self.payment.version.is_empty()
            || self.payment.version.len() > 20
        {
            bail!("invalid payment network configuration");
        }
        crate::features::identity::wallet(&self.payment.asset)?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn unsafe_operational_overrides_fail_closed() -> anyhow::Result<()> {
        let valid: super::RuntimeConfig = toml::from_str(include_str!("../config.toml"))?;
        valid.validate()?;
        let mut config = valid.clone();
        config.http.concurrency = 0;
        assert!(config.validate().is_err());
        let mut config = valid.clone();
        config.worker.stale_after_seconds = 1;
        assert!(config.validate().is_err());
        let mut config = valid;
        config.payment.asset = "http://evil.example".into();
        assert!(config.validate().is_err());
        Ok(())
    }
}
