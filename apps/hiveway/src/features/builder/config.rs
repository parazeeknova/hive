use anyhow::{Context, bail};
#[derive(Clone)]
pub struct Config {
    pub url: String,
    pub api_key: String,
    pub model: String,
    pub reasoning_effort: Option<String>,
    pub owner_daily: u64,
    pub global_daily: u64,
}
impl Config {
    /// Load server-only provider settings; partial configuration is an error.
    /// # Errors
    /// Rejects incomplete credentials, insecure remote URLs and excessive quotas.
    pub fn load() -> anyhow::Result<Option<Self>> {
        let value = |key| std::env::var(key).ok().filter(|v| !v.is_empty());
        let (url, api_key, model) = (
            value("HIVE_BUILDER_API_URL"),
            value("HIVE_BUILDER_API_KEY"),
            value("HIVE_BUILDER_MODEL"),
        );
        if url.is_none() && api_key.is_none() && model.is_none() {
            return Ok(None);
        }
        let url = url.context("HIVE_BUILDER_API_URL is required")?;
        let parsed = reqwest::Url::parse(&url)?;
        let local = matches!(parsed.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if !(parsed.scheme() == "https" || local && parsed.scheme() == "http")
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            bail!("Builder endpoint must use HTTPS outside loopback and cannot embed credentials");
        }
        let number = |name, default| -> anyhow::Result<u64> {
            let n = value(name)
                .map(|v| v.parse())
                .transpose()?
                .unwrap_or(default);
            if !(1..=1000).contains(&n) {
                bail!("Builder quota must be 1..1000");
            }
            Ok(n)
        };
        let reasoning_effort = value("HIVE_BUILDER_REASONING_EFFORT");
        if reasoning_effort.as_deref().is_some_and(|effort| {
            !matches!(
                effort,
                "minimal" | "low" | "medium" | "high" | "xhigh" | "max"
            )
        }) {
            bail!("Invalid builder reasoning effort");
        }
        Ok(Some(Self {
            reasoning_effort,
            url,
            api_key: api_key.context("HIVE_BUILDER_API_KEY is required")?,
            model: model.context("HIVE_BUILDER_MODEL is required")?,
            owner_daily: number("HIVE_BUILDER_ACCOUNT_DAILY_LIMIT", 3)?,
            global_daily: number("HIVE_BUILDER_GLOBAL_DAILY_LIMIT", 20)?,
        }))
    }
}
