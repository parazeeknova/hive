use crate::config::{Catalog, Config, validate_url};
use anyhow::{Result, bail, ensure};
use serde_json::{Value, json};
use std::time::Duration;

pub const RESPONSE_BYTES: usize = 2_097_152;
pub struct Hive {
    pub http: reqwest::Client,
    url: reqwest::Url,
    key: String,
}
pub async fn bounded(mut response: reqwest::Response, limit: usize) -> Result<Value> {
    ensure!(
        response.status().is_success(),
        "HTTP request failed ({})",
        response.status()
    );
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        ensure!(
            bytes.len() + chunk.len() <= limit,
            "response exceeds size limit"
        );
        bytes.extend_from_slice(&chunk);
    }
    Ok(serde_json::from_slice(&bytes)?)
}
impl Hive {
    pub fn new(config: &Config) -> Result<Self> {
        let key = std::fs::read_to_string(&config.key_file)?.trim().to_owned();
        ensure!(
            !key.is_empty() && !key.contains(['\r', '\n']),
            "invalid Hive key file"
        );
        Ok(Self {
            http: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(Duration::from_secs(10))
                .build()?,
            url: validate_url(&config.hive)?,
            key,
        })
    }
    pub async fn catalog(&self) -> Result<Catalog> {
        let value = bounded(
            self.http
                .get(self.url.join("/api/config")?)
                .timeout(Duration::from_secs(15))
                .send()
                .await?,
            RESPONSE_BYTES,
        )
        .await?;
        let catalog: Catalog = serde_json::from_value(value)?;
        ensure!(
            (1..=RESPONSE_BYTES).contains(&catalog.connectors.result_bytes)
                && (1..=RESPONSE_BYTES).contains(&catalog.connectors.argument_bytes)
                && catalog.connectors.poll_wait_seconds <= 60
                && (1..=7200).contains(&catalog.connectors.job_ttl_seconds),
            "invalid published connector limits"
        );
        Ok(catalog)
    }
    pub async fn operation(
        &self,
        name: &str,
        arguments: Value,
        timeout: Duration,
    ) -> Result<Value> {
        let response = self.http.post(self.url.clone()).bearer_auth(&self.key)
            .header("MCP-Protocol-Version", "2025-11-25")
            .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":arguments}}))
            .timeout(timeout).send().await?;
        let body = bounded(response, RESPONSE_BYTES).await?;
        ensure!(body.get("error").is_none(), "Hive RPC rejected {name}");
        let result = &body["result"];
        if result["isError"] == true {
            let code = result["structuredContent"]["error"]
                .as_str()
                .unwrap_or("tool_error");
            bail!("Hive operation {name}: {code}");
        }
        ensure!(
            result["structuredContent"].is_object(),
            "Hive operation returned no structured result"
        );
        Ok(result["structuredContent"].clone())
    }
}
