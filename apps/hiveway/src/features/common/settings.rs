use anyhow::{Context, bail};
use std::env;

pub struct Settings {
    pub runtime: crate::runtime::RuntimeConfig,
    pub public_url: String,
    pub auth_domain: String,
    pub auth_uri: String,
    pub facilitator_url: String,
    pub rpc_url: String,
    pub dodo_api_url: String,
    pub dodo_api_key: Option<String>,
    pub dodo_webhook_secret: Option<String>,
    pub dodo_product_id: Option<String>,
}
impl Settings {
    /// # Errors
    /// Rejects malformed endpoint URLs and insecure nonlocal endpoints.
    pub fn from_env() -> anyhow::Result<Self> {
        let runtime = crate::runtime::RuntimeConfig::load()?;
        let public_url = env::var("HIVE_PUBLIC_URL").unwrap_or_else(|_| runtime.public_url.clone());
        let auth_uri = env::var("HIVE_AUTH_ORIGIN").unwrap_or_else(|_| runtime.auth_origin.clone());
        let public_url = validate_url(&public_url)?;
        let auth_uri = validate_url(&auth_uri)?;
        if reqwest::Url::parse(&auth_uri)?.path() != "/" {
            bail!("HIVE_AUTH_ORIGIN must be an origin without a path");
        }
        let auth_domain = auth_uri
            .split_once("://")
            .context("invalid authentication origin")?
            .1
            .to_owned();
        let facilitator_url = validate_url(
            &env::var("X402_FACILITATOR_URL").unwrap_or_else(|_| runtime.facilitator_url.clone()),
        )?;
        let rpc_url = validate_url(
            &env::var("BASE_SEPOLIA_RPC_URL").unwrap_or_else(|_| runtime.rpc_url.clone()),
        )?;
        let optional = |key| env::var(key).ok().filter(|s| !s.is_empty());
        Ok(Self {
            public_url,
            auth_domain,
            auth_uri,
            facilitator_url,
            rpc_url,
            dodo_api_url: {
                let value = validate_url(
                    &env::var("DODO_PAYMENTS_API_URL")
                        .unwrap_or_else(|_| runtime.dodo_api_url.clone()),
                )?;
                let url = reqwest::Url::parse(&value)?;
                if !matches!(
                    url.host_str(),
                    Some("test.dodopayments.com" | "127.0.0.1" | "localhost")
                ) {
                    bail!("Dodo integration requires test mode or a loopback test fixture");
                }
                value
            },
            dodo_api_key: optional("DODO_PAYMENTS_API_KEY"),
            dodo_webhook_secret: optional("DODO_PAYMENTS_WEBHOOK_SECRET"),
            dodo_product_id: optional("DODO_PAYMENTS_PRODUCT_ID"),
            runtime,
        })
    }
}
fn validate_url(value: &str) -> anyhow::Result<String> {
    let url = reqwest::Url::parse(value).context("invalid service URL")?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if (url.scheme() != "https" && !(local && url.scheme() == "http"))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        bail!(
            "service URLs require HTTPS outside loopback and cannot embed credentials or queries"
        );
    }
    Ok(value.trim_end_matches('/').to_owned())
}
