//! Bring-your-own-key providers configured in the dashboard.
//!
//! An account saves its own API key for a published provider (for example
//! Context7 or Tinyfish). The key is encrypted with `HIVE_SECRETS_KEY`
//! (ChaCha20-Poly1305, bound to the account and provider), validated against
//! the provider before it is stored, and never returned again.
//!
//! Saving a key creates a `hosted` connection named after the provider, so its
//! tools appear as `mcp_<slug>__<tool>` and reuse every connector path: queue
//! bounds, tasks, caching of read-only tools, discovery and annotations. Instead
//! of a remote worker, Hiveway claims the job in-process and calls the provider
//! over Streamable HTTP. A Hive Local connection with the same name takes
//! precedence while it is online.
//!
//! Retries are limited to failures where the request provably did not reach the
//! provider: connection errors before sending and an expired MCP session (which
//! the provider rejects before executing). A call that may have executed is
//! never repeated.
use crate::{
    features::{
        common::{
            error::{ApiError, ApiResult},
            model::{Credential, now},
        },
        connections, identity,
        integrations::{self, Integration, Upstream},
        realtime::{self, job_key},
    },
    state::AppState,
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use chacha20poly1305::{
    ChaCha20Poly1305, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};
use uuid::Uuid;

/// Connection feature marking rows served by Hiveway itself.
pub const HOSTED: &str = "hosted";
const PROTOCOL: &str = "2025-11-25";

fn statement(sql: &str, values: Vec<sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}
fn limits(state: &AppState) -> &crate::runtime::Providers {
    &state.settings.runtime.providers
}

// --- Key storage -----------------------------------------------------------

fn cipher(state: &AppState) -> ApiResult<ChaCha20Poly1305> {
    let key = state
        .settings
        .secrets_key
        .ok_or_else(|| ApiError::unavailable("provider_keys_not_configured"))?;
    Ok(ChaCha20Poly1305::new(&key.into()))
}
fn associated(owner: &str, slug: &str) -> Vec<u8> {
    format!("hive-provider-key\0{owner}\0{slug}").into_bytes()
}
fn seal(state: &AppState, owner: &str, slug: &str, secret: &str) -> ApiResult<(Vec<u8>, Vec<u8>)> {
    let nonce: [u8; 12] = rand::random();
    let aad = associated(owner, slug);
    let ciphertext = cipher(state)?
        .encrypt(
            &Nonce::from(nonce),
            Payload {
                msg: secret.as_bytes(),
                aad: &aad,
            },
        )
        .map_err(|_| ApiError::unavailable("provider_key_encryption_failed"))?;
    Ok((ciphertext, nonce.to_vec()))
}
fn unseal(
    state: &AppState,
    owner: &str,
    slug: &str,
    ciphertext: &[u8],
    nonce: &[u8],
) -> ApiResult<String> {
    let nonce: [u8; 12] = nonce
        .try_into()
        .map_err(|_| ApiError::unavailable("provider_key_unreadable"))?;
    let aad = associated(owner, slug);
    let plain = cipher(state)?
        .decrypt(
            &Nonce::from(nonce),
            Payload {
                msg: ciphertext,
                aad: &aad,
            },
        )
        .map_err(|_| ApiError::unavailable("provider_key_unreadable_resave_it"))?;
    String::from_utf8(plain).map_err(|_| ApiError::unavailable("provider_key_unreadable"))
}
async fn stored_key(state: &AppState, owner: &str, slug: &str) -> ApiResult<Option<String>> {
    let row = state
        .postgres
        .query_one_raw(statement(
            "SELECT ciphertext, nonce FROM account_provider_keys WHERE owner=$1 AND slug=$2",
            vec![owner.into(), slug.into()],
        ))
        .await?;
    row.map(|row| {
        let ciphertext: Vec<u8> = row.try_get("", "ciphertext")?;
        let nonce: Vec<u8> = row.try_get("", "nonce")?;
        unseal(state, owner, slug, &ciphertext, &nonce)
    })
    .transpose()
}

// --- Providers ---------------------------------------------------------------

struct Provider {
    integration: &'static Integration,
    upstream: &'static Upstream,
    url: String,
}
fn provider(state: &AppState, slug: &str) -> ApiResult<Provider> {
    let integration = integrations::registry()
        .iter()
        .find(|item| item.slug == slug && item.byok)
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "unknown_provider"))?;
    let upstream = integration
        .upstream
        .as_ref()
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "unknown_provider"))?;
    let url = state
        .settings
        .provider_urls
        .get(slug)
        .cloned()
        .unwrap_or_else(|| upstream.url.clone());
    Ok(Provider {
        integration,
        upstream,
        url,
    })
}
fn auth_value(upstream: &Upstream, key: &str) -> String {
    if upstream.auth == "bearer" && !key.starts_with("Bearer ") {
        format!("Bearer {key}")
    } else {
        key.to_owned()
    }
}
fn hint(key: &str) -> String {
    let tail: String = key
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("••••{tail}")
}

// --- Streamable HTTP client ----------------------------------------------

/// Why an upstream request failed, and whether it may have executed.
#[derive(Debug)]
pub enum UpstreamError {
    /// The provider rejected the key.
    Unauthorized,
    /// The provider asked us to slow down.
    RateLimited(Option<u64>),
    /// The MCP session expired; the provider did not process the request.
    SessionExpired,
    /// Nothing reached the provider (DNS, connect, TLS).
    Unreachable,
    /// The request may have reached the provider; its outcome is unknown.
    Uncertain,
    /// The provider answered with something that is not MCP.
    Protocol(&'static str),
    /// The provider answered with a JSON-RPC error.
    Rpc(Value),
}
impl UpstreamError {
    fn message(&self, name: &str) -> String {
        match self {
            Self::Unauthorized => {
                format!("{name} rejected the saved API key. Update it in the Hive dashboard.")
            }
            Self::RateLimited(Some(seconds)) => {
                format!("{name} is rate limiting this key; try again in {seconds} seconds.")
            }
            Self::RateLimited(None) => {
                format!("{name} is rate limiting this key; try again shortly.")
            }
            Self::SessionExpired | Self::Unreachable => {
                format!("Hive could not reach {name}; nothing was sent.")
            }
            Self::Uncertain => format!(
                "The {name} call failed after it was sent; its outcome is unknown. Do not repeat mutations automatically."
            ),
            Self::Protocol(reason) => format!("{name} returned an invalid response ({reason})."),
            Self::Rpc(error) => format!(
                "{name} error: {}",
                error["message"].as_str().unwrap_or("request failed")
            ),
        }
    }
}
fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        // Per-request timeouts: provider calls can legitimately run for minutes.
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .pool_idle_timeout(Duration::from_secs(90))
            .user_agent(concat!("hiveway/", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_default()
    })
}
#[derive(Clone)]
struct Session {
    id: Option<String>,
    created: Instant,
    fingerprint: [u8; 8],
}
type Sessions = HashMap<(String, String), Session>;
fn sessions() -> &'static Mutex<Sessions> {
    static SESSIONS: OnceLock<Mutex<Sessions>> = OnceLock::new();
    SESSIONS.get_or_init(Mutex::default)
}
fn fingerprint(key: &str) -> [u8; 8] {
    let digest = Sha256::digest(key.as_bytes());
    let mut out = [0; 8];
    out.copy_from_slice(&digest[..8]);
    out
}
/// Parse one MCP response body: plain JSON or a server-sent event stream.
/// Returns the message answering `id` and any server-to-client requests.
fn parse_body(
    content_type: &str,
    body: &[u8],
    id: &Value,
) -> Result<(Option<Value>, Vec<Value>), UpstreamError> {
    let messages: Vec<Value> = if content_type.starts_with("text/event-stream") {
        String::from_utf8_lossy(body)
            .replace("\r\n", "\n")
            .split("\n\n")
            .filter_map(|event| {
                let data: Vec<&str> = event
                    .lines()
                    .filter_map(|line| line.strip_prefix("data:").map(str::trim_start))
                    .collect();
                (!data.is_empty()).then(|| data.join("\n"))
            })
            .filter_map(|data| serde_json::from_str::<Value>(&data).ok())
            .flat_map(|value| match value {
                Value::Array(items) => items,
                other => vec![other],
            })
            .collect()
    } else if body.is_empty() {
        Vec::new()
    } else {
        match serde_json::from_slice::<Value>(body)
            .map_err(|_| UpstreamError::Protocol("invalid JSON"))?
        {
            Value::Array(items) => items,
            other => vec![other],
        }
    };
    let answer = messages
        .iter()
        .find(|m| &m["id"] == id && m.get("method").is_none())
        .cloned();
    let requests = messages
        .into_iter()
        .filter(|m| m.get("method").is_some() && m.get("id").is_some())
        .collect();
    Ok((answer, requests))
}
struct Call<'a> {
    provider: &'a Provider,
    key: &'a str,
    session: Option<&'a str>,
    timeout: Duration,
    limit: usize,
}
async fn post(
    call: &Call<'_>,
    body: &Value,
) -> Result<(Option<Value>, Option<String>), UpstreamError> {
    let mut request = http()
        .post(&call.provider.url)
        .timeout(call.timeout)
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", PROTOCOL)
        .header(
            call.provider.upstream.header.as_str(),
            auth_value(call.provider.upstream, call.key),
        )
        .json(body);
    if let Some(session) = call.session {
        request = request.header("mcp-session-id", session);
    }
    let mut response = request.send().await.map_err(|error| {
        if error.is_connect() || error.is_builder() {
            UpstreamError::Unreachable
        } else {
            UpstreamError::Uncertain
        }
    })?;
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(UpstreamError::Unauthorized);
    }
    if status == reqwest::StatusCode::NOT_FOUND && call.session.is_some() {
        return Err(UpstreamError::SessionExpired);
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let retry = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok());
        return Err(UpstreamError::RateLimited(retry));
    }
    if status.is_server_error() {
        return Err(UpstreamError::Uncertain);
    }
    if !status.is_success() {
        return Err(UpstreamError::Protocol("unexpected HTTP status"));
    }
    let session = response
        .headers()
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut bytes = Vec::new();
    let id = body["id"].clone();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| UpstreamError::Uncertain)?
    {
        bytes.extend_from_slice(&chunk);
        if bytes.len() > call.limit {
            return Err(UpstreamError::Protocol("response too large"));
        }
        // Stop reading a stream as soon as our answer has arrived.
        if content_type.starts_with("text/event-stream")
            && let Ok((Some(_), _)) = parse_body(&content_type, &bytes, &id)
        {
            break;
        }
    }
    let (answer, requests) = parse_body(&content_type, &bytes, &id)?;
    // Hiveway cannot show server requests (elicitation, sampling) to a user;
    // decline them so the provider does not wait.
    for server_request in requests {
        let decline =
            json!({"jsonrpc":"2.0","id":server_request["id"],"result":{"action":"decline"}});
        let _ = Box::pin(post(call, &decline)).await;
    }
    Ok((answer, session))
}
async fn initialize(
    provider: &Provider,
    key: &str,
    timeout: Duration,
    limit: usize,
) -> Result<Session, UpstreamError> {
    let call = Call {
        provider,
        key,
        session: None,
        timeout,
        limit,
    };
    let (answer, id) = post(
        &call,
        &json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":PROTOCOL,"capabilities":{},"clientInfo":{"name":"hiveway","version":env!("CARGO_PKG_VERSION")}}}),
    )
    .await?;
    let answer = answer.ok_or(UpstreamError::Protocol("no initialize result"))?;
    if answer.get("error").is_some() {
        return Err(UpstreamError::Rpc(answer["error"].clone()));
    }
    let session = Session {
        id,
        created: Instant::now(),
        fingerprint: fingerprint(key),
    };
    let call = Call {
        session: session.id.as_deref(),
        ..call
    };
    post(
        &call,
        &json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
    )
    .await?;
    Ok(session)
}
/// One upstream request with session reuse and only provably safe retries.
async fn call_upstream(
    state: &AppState,
    owner: &str,
    provider: &Provider,
    key: &str,
    method: &str,
    params: Value,
    timeout: Duration,
) -> Result<Value, UpstreamError> {
    let limits = limits(state);
    let limit = state.settings.runtime.connectors.result_bytes * 2;
    let cache_key = (owner.to_owned(), provider.integration.slug.clone());
    let ttl = Duration::from_secs(limits.session_ttl_seconds);
    // Concurrent calls share one session, so a single expiry can hit many of
    // them at once and a renewed session can expire again; each retry is safe
    // because the provider refused the request before executing it.
    let mut expired = 0;
    let mut attempt = 0;
    // Forget the cached session only if it is still the one that failed, so a
    // session another call just created is kept.
    let forget = |failed: &Option<String>| {
        if let Ok(mut map) = sessions().lock()
            && map.get(&cache_key).is_some_and(|s| &s.id == failed)
        {
            map.remove(&cache_key);
        }
    };
    loop {
        let cached = sessions().lock().ok().and_then(|map| {
            map.get(&cache_key)
                .filter(|s| s.created.elapsed() < ttl && s.fingerprint == fingerprint(key))
                .cloned()
        });
        let session = match cached {
            Some(session) => session,
            None => {
                match initialize(provider, key, timeout.min(Duration::from_secs(30)), limit).await {
                    Ok(session) => {
                        if let Ok(mut map) = sessions().lock() {
                            if map.len() >= limits.sessions_cached {
                                map.retain(|_, s| s.created.elapsed() < ttl);
                            }
                            if map.len() < limits.sessions_cached {
                                map.insert(cache_key.clone(), session.clone());
                            }
                        }
                        session
                    }
                    Err(UpstreamError::Unreachable) if attempt < limits.connect_retries => {
                        attempt += 1;
                        tokio::time::sleep(Duration::from_millis(200 << attempt)).await;
                        continue;
                    }
                    Err(error) => return Err(error),
                }
            }
        };
        let call = Call {
            provider,
            key,
            session: session.id.as_deref(),
            timeout,
            limit,
        };
        let id = json!(Uuid::new_v4().to_string());
        match post(
            &call,
            &json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}),
        )
        .await
        {
            Ok((Some(answer), _)) => {
                if answer.get("error").is_some() {
                    return Err(UpstreamError::Rpc(answer["error"].clone()));
                }
                return Ok(answer["result"].clone());
            }
            Ok((None, _)) => return Err(UpstreamError::Protocol("no result")),
            Err(UpstreamError::SessionExpired) if expired < 3 => {
                expired += 1;
                forget(&session.id);
            }
            Err(UpstreamError::Unreachable) if attempt < limits.connect_retries => {
                attempt += 1;
                tokio::time::sleep(Duration::from_millis(200 << attempt)).await;
            }
            Err(error) => {
                if matches!(
                    error,
                    UpstreamError::Unauthorized | UpstreamError::Uncertain
                ) {
                    forget(&session.id);
                }
                return Err(error);
            }
        }
    }
}
/// The provider's tools, with operator-verified read-only hints applied.
async fn list_tools(
    state: &AppState,
    owner: &str,
    provider: &Provider,
    key: &str,
) -> Result<Vec<Value>, UpstreamError> {
    let timeout = Duration::from_secs(limits(state).validation_timeout_seconds);
    let max = state.settings.runtime.connectors.tools_per_connection;
    let mut tools = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..8 {
        let params = cursor
            .as_ref()
            .map_or_else(|| json!({}), |c| json!({"cursor":c}));
        let page =
            call_upstream(state, owner, provider, key, "tools/list", params, timeout).await?;
        for tool in page["tools"].as_array().into_iter().flatten() {
            if tools.len() < max && tool["name"].as_str().is_some_and(connections::valid_name) {
                let mut tool = tool.clone();
                if provider
                    .upstream
                    .read_only_tools
                    .iter()
                    .any(|name| tool["name"] == name.as_str())
                {
                    tool["annotations"]["readOnlyHint"] = json!(true);
                }
                if !tool["inputSchema"].is_object() {
                    tool["inputSchema"] = json!({"type":"object"});
                }
                tools.push(tool);
            }
        }
        cursor = page["nextCursor"].as_str().map(str::to_owned);
        if cursor.is_none() {
            break;
        }
    }
    Ok(tools)
}

// --- Account API -----------------------------------------------------------

async fn session_owner(state: &AppState, headers: &HeaderMap) -> ApiResult<Credential> {
    // Keys are managed only from a browser session, never by agents.
    identity::authenticate_session(state, headers).await
}
/// Providers an account can configure, with their status. Keys are never returned.
/// # Errors
/// Rejects non-session credentials.
pub async fn list(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let c = session_owner(&state, &headers).await?;
    let rows = state
        .postgres
        .query_all_raw(statement(
            "SELECT k.slug, k.hint, k.tools, k.last_error, k.updated_at, EXISTS(SELECT 1 FROM mcp_connections m WHERE m.owner=k.owner AND m.name=k.slug AND NOT ('hosted'=ANY(m.features)) AND m.updated_at>=$2) AS local FROM account_provider_keys k WHERE k.owner=$1",
            vec![c.wallet.clone().into(), (now() - state.settings.runtime.connectors.online_window_seconds).into()],
        ))
        .await?;
    let mut saved: HashMap<String, Value> = HashMap::new();
    for row in rows {
        let local: bool = row.try_get("", "local")?;
        saved.insert(
            row.try_get("", "slug")?,
            json!({
                "hint": row.try_get::<String>("", "hint")?,
                "tools": row.try_get::<i32>("", "tools")?,
                "last_error": row.try_get::<Option<String>>("", "last_error")?,
                "updated_at": row.try_get::<i64>("", "updated_at")?,
                "served_by": if local { "hive_local" } else { "hive" },
            }),
        );
    }
    let providers: Vec<Value> = integrations::registry()
        .iter()
        .filter(|item| item.byok && item.upstream.is_some())
        .map(|item| {
            json!({
                "slug": item.slug,
                "name": item.name,
                "description": item.description,
                "configured": saved.contains_key(&item.slug),
                "key": saved.get(&item.slug),
            })
        })
        .collect();
    Ok(Json(
        json!({"available": state.settings.secrets_key.is_some(), "providers": providers}),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveKey {
    api_key: String,
}
/// Validate the key against the provider, then store it encrypted and expose its tools.
async fn activate(state: &AppState, c: &Credential, slug: &str, key: &str) -> ApiResult<Value> {
    let provider = provider(state, slug)?;
    let tools = match list_tools(state, &c.wallet, &provider, key).await {
        Ok(tools) if !tools.is_empty() => tools,
        Ok(_) => return Err(ApiError::bad("provider_returned_no_tools")),
        Err(UpstreamError::Unauthorized) => return Err(ApiError::bad("provider_rejected_key")),
        Err(UpstreamError::RateLimited(_)) => {
            return Err(ApiError::new(
                StatusCode::TOO_MANY_REQUESTS,
                "provider_rate_limited",
            ));
        }
        Err(error) => {
            tracing::warn!(provider = slug, ?error, "Provider validation failed");
            return Err(ApiError::unavailable("provider_unreachable"));
        }
    };
    let (ciphertext, nonce) = seal(state, &c.wallet, slug, key)?;
    let at = now();
    let count = i32::try_from(tools.len()).unwrap_or(i32::MAX);
    state.postgres.execute_raw(statement(
        "INSERT INTO account_provider_keys(owner,slug,ciphertext,nonce,hint,tools,last_error,created_at,updated_at) VALUES($1,$2,$3,$4,$5,$6,NULL,$7,$7) ON CONFLICT(owner,slug) DO UPDATE SET ciphertext=EXCLUDED.ciphertext,nonce=EXCLUDED.nonce,hint=EXCLUDED.hint,tools=EXCLUDED.tools,last_error=NULL,updated_at=EXCLUDED.updated_at",
        vec![c.wallet.clone().into(), slug.into(), ciphertext.into(), nonce.into(), hint(key).into(), count.into(), at.into()],
    )).await?;
    connections::upsert_hosted(state, &c.wallet, slug, Value::Array(tools)).await?;
    Ok(json!({"slug":slug,"configured":true,"tools":count,"hint":hint(key)}))
}
/// Save or replace an account's key for a provider.
/// # Errors
/// Rejects malformed keys, keys the provider refuses, and unconfigured deployments.
pub async fn save(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Json(input): Json<SaveKey>,
) -> ApiResult<Json<Value>> {
    let c = session_owner(&state, &headers).await?;
    cipher(&state)?;
    let key = input.api_key.trim();
    if key.is_empty()
        || key.len() > limits(&state).key_bytes
        || key
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace() && ch != ' ')
    {
        return Err(ApiError::bad("invalid_provider_key"));
    }
    Ok(Json(activate(&state, &c, &slug, key).await?))
}
/// Re-read the provider's tools with the stored key.
/// # Errors
/// Rejects unknown providers and accounts without a saved key.
pub async fn refresh(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    let c = session_owner(&state, &headers).await?;
    let key = stored_key(&state, &c.wallet, &slug)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "provider_key_not_saved"))?;
    Ok(Json(activate(&state, &c, &slug, &key).await?))
}
/// Delete the saved key and the provider's hosted connection.
/// # Errors
/// Rejects non-session credentials.
pub async fn remove(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    let c = session_owner(&state, &headers).await?;
    let deleted = state
        .postgres
        .execute_raw(statement(
            "DELETE FROM account_provider_keys WHERE owner=$1 AND slug=$2",
            vec![c.wallet.clone().into(), slug.clone().into()],
        ))
        .await?;
    connections::remove_hosted(&state, &c.wallet, &slug).await?;
    if let Ok(mut map) = sessions().lock() {
        map.remove(&(c.wallet.clone(), slug));
    }
    Ok(Json(json!({"removed":deleted.rows_affected() == 1})))
}

// --- Execution ---------------------------------------------------------------

fn permits(state: &AppState) -> Arc<tokio::sync::Semaphore> {
    static PERMITS: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();
    PERMITS
        .get_or_init(|| Arc::new(tokio::sync::Semaphore::new(limits(state).concurrency)))
        .clone()
}
/// Execute a queued hosted job in the background. The caller waits on the job row.
pub fn spawn(state: &AppState, job: Uuid, connection: Uuid) {
    let state = state.clone();
    tokio::spawn(async move {
        if let Err(error) = run(&state, job, connection).await {
            tracing::warn!(error = error.code, %job, "Hosted provider job failed");
        }
    });
}
fn failed(text: &str) -> Value {
    json!({"content":[{"type":"text","text":text}],"isError":true})
}
async fn run(state: &AppState, job: Uuid, connection: Uuid) -> ApiResult<()> {
    let _permit = permits(state)
        .acquire_owned()
        .await
        .map_err(|_| ApiError::unavailable("provider_capacity"))?;
    let lease = Uuid::new_v4();
    let at = now();
    let Some(claimed) = state
        .postgres
        .query_one_raw(statement(
            "UPDATE mcp_jobs j SET lease=$2, status='working', updated_at=$3 FROM mcp_connections c WHERE j.id=$1 AND c.id=j.connection_id AND j.status='queued' AND j.claim_expires_at>$3 RETURNING j.request, j.expires_at, c.owner, c.name",
            vec![job.into(), lease.into(), at.into()],
        ))
        .await?
    else {
        return Ok(());
    };
    realtime::announce(state, &[job_key(job)]).await;
    let request: Value = claimed.try_get("", "request")?;
    let owner: String = claimed.try_get("", "owner")?;
    let slug: String = claimed.try_get("", "name")?;
    let expires: i64 = claimed.try_get("", "expires_at")?;
    let complete = |payload: Value| {
        let mut args = payload;
        args["job_id"] = json!(job);
        args["lease"] = json!(lease);
        args
    };
    let (Ok(provider), Ok(Some(key))) = (
        provider(state, &slug),
        stored_key(state, &owner, &slug).await,
    ) else {
        let args = complete(
            json!({"result":failed("No usable API key is saved for this provider. Add one in the Hive dashboard.")}),
        );
        return connections::complete_job(state, connection, &args).await;
    };
    let timeout = Duration::from_secs(u64::try_from((expires - now() - 5).max(5)).unwrap_or(5));
    let call = call_upstream(
        state,
        &owner,
        &provider,
        &key,
        "tools/call",
        json!({"name":request["name"],"arguments":request["arguments"]}),
        timeout,
    );
    // The cancellation watch runs as its own task and is only told to stop
    // between queries: dropping a future in the middle of a pool query keeps
    // that connection busy and, under load, starves the pool.
    let (stop, stopped) = tokio::sync::watch::channel(false);
    let (cancel, cancelled) = tokio::sync::oneshot::channel();
    tokio::spawn(watch_cancel(state.clone(), job, stopped, cancel));
    tokio::pin!(call);
    let name = &provider.integration.name;
    let args = tokio::select! {
        outcome = &mut call => match outcome {
            Ok(result) if result.is_object() => complete(json!({"result":result})),
            Ok(_) => complete(json!({"result":failed(&UpstreamError::Protocol("empty result").message(name))})),
            Err(error) => {
                if matches!(error, UpstreamError::Unauthorized) {
                    state.postgres.execute_raw(statement(
                        "UPDATE account_provider_keys SET last_error='rejected_by_provider', updated_at=$3 WHERE owner=$1 AND slug=$2",
                        vec![owner.clone().into(), slug.clone().into(), now().into()],
                    )).await?;
                }
                complete(json!({"result":failed(&error.message(name))}))
            }
        },
        Ok(()) = cancelled => complete(json!({"cancelled":true,"result":failed("Cancelled at the caller's request.")})),
    };
    let _ = stop.send(true);
    connections::complete_job(state, connection, &args).await
}
/// Signal `cancel` when the caller cancels the job (or it disappears). Checks
/// on every job wakeup and at least every five seconds until `stop` is set.
async fn watch_cancel(
    state: AppState,
    job: Uuid,
    mut stop: tokio::sync::watch::Receiver<bool>,
    cancel: tokio::sync::oneshot::Sender<()>,
) {
    let keys = [job_key(job)];
    loop {
        let notify = state.realtime.watch(&keys);
        let notified = notify.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        let row = state
            .postgres
            .query_one_raw(statement(
                "SELECT cancel_requested FROM mcp_jobs WHERE id=$1",
                vec![job.into()],
            ))
            .await;
        let requested = match row {
            Ok(Some(row)) => row.try_get::<bool>("", "cancel_requested").unwrap_or(false),
            Ok(None) => true,
            // A transient database error is retried on the next wakeup.
            Err(_) => false,
        };
        if requested {
            let _ = cancel.send(());
            return;
        }
        if *stop.borrow() {
            return;
        }
        tokio::select! {
            () = &mut notified => {}
            () = tokio::time::sleep(Duration::from_secs(5)) => {}
            _ = stop.changed() => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn streamed_and_plain_bodies_yield_the_matching_answer() -> Result<(), String> {
        let id = json!("a");
        let sse = b"event: message\ndata: {\"jsonrpc\":\"2.0\",\"method\":\"notifications/progress\",\"params\":{}}\n\ndata: {\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"elicitation/create\",\"params\":{}}\n\ndata: {\"jsonrpc\":\"2.0\",\"id\":\"a\",\"result\":{\"ok\":true}}\n\n";
        let (answer, requests) =
            parse_body("text/event-stream", sse, &id).map_err(|e| format!("{e:?}"))?;
        assert_eq!(answer.map(|a| a["result"]["ok"].clone()), Some(json!(true)));
        assert_eq!(
            requests.len(),
            1,
            "server requests are answered with a decline"
        );
        let (answer, _) = parse_body(
            "application/json",
            br#"{"jsonrpc":"2.0","id":"a","result":{}}"#,
            &id,
        )
        .map_err(|e| format!("{e:?}"))?;
        assert!(answer.is_some());
        assert!(parse_body("application/json", b"<html>", &id).is_err());
        Ok(())
    }
    #[test]
    fn hints_and_auth_headers_never_echo_whole_keys() {
        assert_eq!(hint("ctx7sk-1234abcd"), "••••abcd");
        let bearer = Upstream {
            url: String::new(),
            auth: "bearer".into(),
            header: "Authorization".into(),
            read_only_tools: vec![],
        };
        assert_eq!(auth_value(&bearer, "k"), "Bearer k");
        assert_eq!(auth_value(&bearer, "Bearer k"), "Bearer k");
        let raw = Upstream {
            auth: "header".into(),
            ..bearer
        };
        assert_eq!(auth_value(&raw, "k"), "k");
    }
    #[test]
    fn sealed_keys_are_bound_to_owner_and_provider() -> ApiResult<()> {
        let cipher = ChaCha20Poly1305::new(&[7u8; 32].into());
        let nonce = [1u8; 12];
        let aad = associated("github:1", "context7");
        let sealed = cipher
            .encrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: b"secret",
                    aad: &aad,
                },
            )
            .map_err(|_| ApiError::bad("encrypt"))?;
        let other = associated("github:2", "context7");
        assert!(
            cipher
                .decrypt(
                    &Nonce::from(nonce),
                    Payload {
                        msg: &sealed,
                        aad: &other
                    }
                )
                .is_err()
        );
        let opened = cipher
            .decrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: &sealed,
                    aad: &aad,
                },
            )
            .map_err(|_| ApiError::bad("decrypt"))?;
        assert_eq!(opened, b"secret");
        Ok(())
    }
}
