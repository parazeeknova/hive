use anyhow::{Context, bail};
use std::env;

pub struct Settings {
    pub builder: Option<crate::features::builder::config::Config>,
    /// Optional semantic ranking for `search_tools`.
    pub embeddings: Option<crate::features::hiveway::embeddings::Config>,
    /// `HIVE_SECRETS_KEY`: 32-byte key that encrypts provider keys saved in the
    /// dashboard. Without it, dashboard provider keys are disabled.
    pub secrets_key: Option<[u8; 32]>,
    /// Shared services that must be online before a deployment becomes ready.
    pub required_shared_integrations: Vec<String>,
    /// `HIVE_PROVIDER_URL_<SLUG>` overrides for published provider endpoints.
    pub provider_urls: std::collections::HashMap<String, String>,
    /// Optional `HIVE_GITHUB_TOKEN` for rate-limited public repository analysis.
    /// Authenticated analysis must verify repository visibility before fetching contents.
    pub github_token: Option<String>,
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
        runtime.payment.settlement_finality.validate_rpc(&rpc_url)?;
        let optional = |key| env::var(key).ok().filter(|s| !s.is_empty());
        Ok(Self {
            builder: crate::features::builder::config::Config::load()?,
            embeddings: crate::features::hiveway::embeddings::Config::load()?,
            secrets_key: secrets_key()?,
            required_shared_integrations: required_shared_integrations()?,
            provider_urls: provider_urls()?,
            github_token: optional("HIVE_GITHUB_TOKEN"),
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
fn required_shared_integrations() -> anyhow::Result<Vec<String>> {
    let mut names = Vec::new();
    for name in env::var("HIVE_REQUIRED_SHARED_INTEGRATIONS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if !crate::features::integrations::registry()
            .iter()
            .any(|item| item.slug == name && item.shared_owner.is_some())
        {
            bail!("HIVE_REQUIRED_SHARED_INTEGRATIONS must name configured shared integrations");
        }
        if !names.iter().any(|v| v == name) {
            names.push(name.to_owned());
        }
    }
    Ok(names)
}
/// Parse `HIVE_SECRETS_KEY` as 64 hex characters or standard base64 of 32 bytes.
fn secrets_key() -> anyhow::Result<Option<[u8; 32]>> {
    use base64::Engine;
    let Some(raw) = env::var("HIVE_SECRETS_KEY")
        .ok()
        .filter(|v| !v.trim().is_empty())
    else {
        return Ok(None);
    };
    let raw = raw.trim();
    let bytes = if raw.len() == 64 && raw.bytes().all(|b| b.is_ascii_hexdigit()) {
        hex::decode(raw)?
    } else {
        base64::engine::general_purpose::STANDARD
            .decode(raw)
            .context("HIVE_SECRETS_KEY must be 64 hex characters or base64")?
    };
    let key: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("HIVE_SECRETS_KEY must encode exactly 32 bytes"))?;
    if key.iter().all(|b| *b == 0) {
        bail!("HIVE_SECRETS_KEY must not be all zeros");
    }
    Ok(Some(key))
}
/// `HIVE_PROVIDER_URL_<SLUG>` overrides, for staging or self-hosted upstreams.
fn provider_urls() -> anyhow::Result<std::collections::HashMap<String, String>> {
    let mut urls = std::collections::HashMap::new();
    for item in crate::features::integrations::registry() {
        let key = format!(
            "HIVE_PROVIDER_URL_{}",
            item.slug.to_ascii_uppercase().replace('-', "_")
        );
        if let Some(value) = env::var(key).ok().filter(|v| !v.is_empty()) {
            urls.insert(item.slug.clone(), validate_url(&value)?);
        }
    }
    Ok(urls)
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
