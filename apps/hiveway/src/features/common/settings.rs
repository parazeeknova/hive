use anyhow::{Context, bail};
use std::env;

pub struct Settings {
    pub public_url: String,
    pub auth_domain: String,
    pub auth_uri: String,
    pub facilitator_url: String,
    pub rpc_url: String,
    pub github_token: Option<String>,
    pub dodo_api_key: Option<String>,
    pub dodo_webhook_secret: Option<String>,
    pub dodo_product_id: Option<String>,
}
impl Settings {
    /// # Errors
    /// Rejects malformed endpoint URLs and insecure nonlocal endpoints.
    pub fn from_env() -> anyhow::Result<Self> {
        let public_url =
            env::var("HIVE_PUBLIC_URL").unwrap_or_else(|_| "http://127.0.0.1:3001".into());
        let auth_uri =
            env::var("HIVE_AUTH_ORIGIN").unwrap_or_else(|_| "http://127.0.0.1:3000".into());
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
            &env::var("X402_FACILITATOR_URL")
                .unwrap_or_else(|_| "https://x402.org/facilitator".into()),
        )?;
        let rpc_url = validate_url(
            &env::var("BASE_SEPOLIA_RPC_URL").unwrap_or_else(|_| "https://sepolia.base.org".into()),
        )?;
        let optional = |key| env::var(key).ok().filter(|s| !s.is_empty());
        Ok(Self {
            public_url,
            auth_domain,
            auth_uri,
            facilitator_url,
            rpc_url,
            github_token: optional("GITHUB_TOKEN"),
            dodo_api_key: optional("DODO_PAYMENTS_API_KEY"),
            dodo_webhook_secret: optional("DODO_PAYMENTS_WEBHOOK_SECRET"),
            dodo_product_id: optional("DODO_PAYMENTS_PRODUCT_ID"),
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
