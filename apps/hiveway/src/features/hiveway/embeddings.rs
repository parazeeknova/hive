//! Optional semantic ranking for `search_tools` through any OpenAI-compatible
//! `/embeddings` endpoint. Vectors are cached in Postgres by content hash, so a
//! tool description is embedded once per model, not once per search. Any
//! provider failure degrades to lexical ranking; it never fails a search.
use crate::{
    features::common::{error::ApiResult, model::now},
    state::AppState,
};
use anyhow::{Context, bail};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Clone)]
pub struct Config {
    pub url: String,
    pub api_key: String,
    pub model: String,
}
impl Config {
    /// Load `HIVE_EMBEDDINGS_API_URL`, `HIVE_EMBEDDINGS_API_KEY` and
    /// `HIVE_EMBEDDINGS_MODEL`. All three or none.
    /// # Errors
    /// Rejects partial configuration and insecure remote endpoints.
    pub fn load() -> anyhow::Result<Option<Self>> {
        let value = |key| std::env::var(key).ok().filter(|v| !v.is_empty());
        let (url, api_key, model) = (
            value("HIVE_EMBEDDINGS_API_URL"),
            value("HIVE_EMBEDDINGS_API_KEY"),
            value("HIVE_EMBEDDINGS_MODEL"),
        );
        if url.is_none() && api_key.is_none() && model.is_none() {
            return Ok(None);
        }
        let url = url.context("HIVE_EMBEDDINGS_API_URL is required")?;
        let parsed = reqwest::Url::parse(&url)?;
        let local = matches!(parsed.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if !(parsed.scheme() == "https" || local && parsed.scheme() == "http")
            || !parsed.username().is_empty()
            || parsed.query().is_some()
        {
            bail!(
                "Embeddings endpoint must use HTTPS outside loopback and cannot embed credentials"
            );
        }
        let model = model.context("HIVE_EMBEDDINGS_MODEL is required")?;
        if model.len() > 200 {
            bail!("Embeddings model name is too long");
        }
        Ok(Some(Self {
            url,
            api_key: api_key.context("HIVE_EMBEDDINGS_API_KEY is required")?,
            model,
        }))
    }
}

fn digest(model: &str, text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(model.as_bytes());
    hasher.update([0]);
    hasher.update(text.as_bytes());
    hex::encode(hasher.finalize())
}
async fn request(
    state: &AppState,
    config: &Config,
    inputs: &[&str],
) -> anyhow::Result<Vec<Vec<f32>>> {
    let timeout =
        std::time::Duration::from_millis(state.settings.runtime.discovery.embedding_timeout_ms);
    let response: Value = state
        .http
        .post(&config.url)
        .bearer_auth(&config.api_key)
        .timeout(timeout)
        .json(&json!({"model":config.model,"input":inputs}))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let mut rows: Vec<(usize, Vec<f32>)> = response["data"]
        .as_array()
        .context("embeddings response without data")?
        .iter()
        .enumerate()
        .map(|(position, item)| {
            let index = item["index"]
                .as_u64()
                .and_then(|i| usize::try_from(i).ok())
                .unwrap_or(position);
            #[allow(clippy::cast_possible_truncation)]
            let vector = item["embedding"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_f64)
                .map(|v| v as f32)
                .collect();
            (index, vector)
        })
        .collect();
    rows.sort_by_key(|(index, _)| *index);
    if rows.len() != inputs.len() || rows.iter().any(|(_, v)| v.is_empty() || v.len() > 8192) {
        bail!("embeddings response does not match the request");
    }
    Ok(rows.into_iter().map(|(_, v)| v).collect())
}

/// Vectors for `texts`, reading the cache first and embedding only misses.
/// Returns `None` when embeddings are not configured or the provider failed.
pub async fn vectors(state: &AppState, texts: &[String]) -> Option<Vec<Vec<f32>>> {
    let config = state.settings.embeddings.as_ref()?;
    match cached_vectors(state, config, texts).await {
        Ok(vectors) => Some(vectors),
        Err(error) => {
            tracing::warn!(%error, "Semantic tool ranking unavailable; using lexical ranking");
            None
        }
    }
}
async fn cached_vectors(
    state: &AppState,
    config: &Config,
    texts: &[String],
) -> anyhow::Result<Vec<Vec<f32>>> {
    let hashes: Vec<String> = texts.iter().map(|t| digest(&config.model, t)).collect();
    let rows = state
        .postgres
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT text_hash, vector FROM tool_embeddings WHERE model=$1 AND text_hash=ANY($2)",
            vec![config.model.clone().into(), hashes.clone().into()],
        ))
        .await?;
    let mut known: HashMap<String, Vec<f32>> = HashMap::new();
    for row in rows {
        known.insert(row.try_get("", "text_hash")?, row.try_get("", "vector")?);
    }
    let missing: Vec<usize> = (0..texts.len())
        .filter(|i| !known.contains_key(&hashes[*i]))
        .collect();
    let batch = state.settings.runtime.discovery.embedding_batch;
    for chunk in missing.chunks(batch) {
        let inputs: Vec<&str> = chunk.iter().map(|i| texts[*i].as_str()).collect();
        let vectors = request(state, config, &inputs).await?;
        for (index, vector) in chunk.iter().zip(vectors) {
            state
                .postgres
                .execute_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    "INSERT INTO tool_embeddings(text_hash,model,vector,created_at) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING",
                    vec![hashes[*index].clone().into(), config.model.clone().into(), vector.clone().into(), now().into()],
                ))
                .await?;
            known.insert(hashes[*index].clone(), vector);
        }
    }
    hashes
        .iter()
        .map(|hash| {
            known
                .get(hash)
                .cloned()
                .context("embedding missing after fetch")
        })
        .collect()
}
/// Cosine similarity in `[0, 1]` (negative similarity counts as unrelated).
#[must_use]
pub fn similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let (mut dot, mut na, mut nb) = (0.0f64, 0.0f64, 0.0f64);
    for (x, y) in a.iter().zip(b) {
        dot += f64::from(*x) * f64::from(*y);
        na += f64::from(*x) * f64::from(*x);
        nb += f64::from(*y) * f64::from(*y);
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    (dot / (na.sqrt() * nb.sqrt())).clamp(0.0, 1.0)
}
/// Drop cached vectors for models no longer configured.
/// # Errors
/// Returns database errors.
pub async fn prune(state: &AppState) -> ApiResult<()> {
    let model = state.settings.embeddings.as_ref().map(|c| c.model.clone());
    state
        .postgres
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "DELETE FROM tool_embeddings WHERE $1::text IS NULL OR model<>$1",
            vec![model.into()],
        ))
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn similarity_is_bounded_and_scale_free() {
        assert!((super::similarity(&[1.0, 0.0], &[2.0, 0.0]) - 1.0).abs() < 1e-9);
        assert_eq!(super::similarity(&[1.0, 0.0], &[-1.0, 0.0]), 0.0);
        assert_eq!(super::similarity(&[1.0], &[1.0, 2.0]), 0.0);
        assert_eq!(super::similarity(&[0.0, 0.0], &[1.0, 2.0]), 0.0);
    }
}
