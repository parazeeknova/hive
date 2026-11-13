//! MCP authorization server (OAuth 2.1 with PKCE) for remote MCP clients.
//!
//! - `/.well-known/oauth-protected-resource[/mcp]` (RFC 9728) points clients here.
//! - `/.well-known/oauth-authorization-server` (RFC 8414) advertises endpoints.
//! - Clients register with Client ID Metadata Documents (an HTTPS `client_id`
//!   Hive fetches with SSRF protections) or, for older clients, RFC 7591 dynamic
//!   registration. Only public clients with PKCE S256 are supported.
//! - Nectar renders consent for the signed-in GitHub session and calls
//!   [`authorize`]; the redirect carries `iss` (RFC 9207).
//! - Access tokens are ordinary scoped `hwa_` credentials, so every existing
//!   authorization check applies unchanged. Refresh tokens rotate on every use
//!   and reuse revokes the whole grant.
use crate::{
    entities::agent_credentials,
    features::{
        common::{
            error::{ApiError, ApiResult},
            model::{Credential, SESSION_SCOPES, now, scope_description},
        },
        identity,
    },
    state::AppState,
};
use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DatabaseTransaction, DbBackend, Set, Statement,
    TransactionTrait,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, net::IpAddr};
use uuid::Uuid;

fn statement(sql: &str, values: Vec<sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}
fn resource(state: &AppState) -> String {
    format!("{}/mcp", state.settings.public_url)
}
fn issuer(state: &AppState) -> String {
    state.settings.public_url.clone()
}
/// `WWW-Authenticate` for MCP 401/403 responses, pointing at resource metadata.
#[must_use]
pub fn challenge(state: &AppState, insufficient_scope: Option<&str>) -> HeaderValue {
    let mut value = format!(
        "Bearer resource_metadata=\"{}/.well-known/oauth-protected-resource/mcp\"",
        state.settings.public_url
    );
    if let Some(scope) = insufficient_scope {
        use std::fmt::Write;
        let _ = write!(value, ", error=\"insufficient_scope\", scope=\"{scope}\"");
    }
    HeaderValue::from_str(&value).unwrap_or_else(|_| HeaderValue::from_static("Bearer"))
}

/// RFC 9728 protected resource metadata.
pub async fn protected_resource(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "resource": resource(&state),
        "authorization_servers": [issuer(&state)],
        "scopes_supported": SESSION_SCOPES,
        "bearer_methods_supported": ["header"],
        "resource_name": "Hive",
        "resource_documentation": format!("{}/docs", state.settings.auth_uri),
    }))
}
/// RFC 8414 authorization server metadata.
pub async fn authorization_server(State(state): State<AppState>) -> Json<Value> {
    let public = &state.settings.public_url;
    Json(json!({
        "issuer": issuer(&state),
        "authorization_endpoint": format!("{}/oauth/authorize", state.settings.auth_uri),
        "token_endpoint": format!("{public}/oauth/token"),
        "registration_endpoint": format!("{public}/oauth/register"),
        "revocation_endpoint": format!("{public}/oauth/revoke"),
        "response_types_supported": ["code"],
        "response_modes_supported": ["query"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none"],
        "revocation_endpoint_auth_methods_supported": ["none"],
        "scopes_supported": SESSION_SCOPES,
        "client_id_metadata_document_supported": true,
        "authorization_response_iss_parameter_supported": true,
        "service_documentation": format!("{}/docs", state.settings.auth_uri),
    }))
}

/// RFC 6749 error response.
fn oauth_error(status: StatusCode, error: &'static str, description: &str) -> Response {
    (
        status,
        Json(json!({"error":error,"error_description":description})),
    )
        .into_response()
}
type OAuthResult<T> = Result<T, Box<Response>>;
// Boxed: these feed `OAuthResult` errors, which must stay small.
#[allow(clippy::unnecessary_box_returns)]
fn db(error: impl std::fmt::Display) -> Box<Response> {
    tracing::error!(%error, "OAuth storage failure");
    Box::new(oauth_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "server_error",
        "Storage unavailable",
    ))
}

/// Redirect URIs: HTTPS anywhere, HTTP only on loopback (RFC 8252), or a
/// private-use scheme for native apps. Never script-capable schemes.
fn valid_redirect(uri: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(uri) else {
        return false;
    };
    if uri.len() > 2048
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return false;
    }
    match url.scheme() {
        "https" => url.host_str().is_some(),
        "http" => matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]")),
        scheme => {
            !matches!(
                scheme,
                "javascript"
                    | "data"
                    | "file"
                    | "vbscript"
                    | "about"
                    | "blob"
                    | "ftp"
                    | "ws"
                    | "wss"
            ) && scheme.len() <= 64
        }
    }
}
/// Exact match, except loopback HTTP redirects may use any port (RFC 8252 §7.3).
fn redirect_matches(registered: &str, presented: &str) -> bool {
    if registered == presented {
        return true;
    }
    let (Ok(a), Ok(b)) = (
        reqwest::Url::parse(registered),
        reqwest::Url::parse(presented),
    ) else {
        return false;
    };
    a.scheme() == "http"
        && b.scheme() == "http"
        && matches!(a.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))
        && a.host_str() == b.host_str()
        && a.path() == b.path()
        && a.query() == b.query()
}
fn clean_name(value: &str, max: usize) -> String {
    value
        .chars()
        .filter(|c| !c.is_control())
        .take(max)
        .collect::<String>()
        .trim()
        .to_owned()
}

/// Every resolved address must be publicly routable before Hive fetches a client document.
fn public_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let [a, b, ..] = v4.octets();
            !(v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_documentation()
                || v4.is_unspecified()
                || v4.is_multicast()
                || a == 0
                || (a == 100 && (64..128).contains(&b))
                || (a == 198 && (b == 18 || b == 19))
                || a >= 240)
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return public_address(IpAddr::V4(v4));
            }
            let first = v6.segments()[0];
            !(v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (first & 0xfe00) == 0xfc00
                || (first & 0xffc0) == 0xfe80
                || first == 0x2001 && v6.segments()[1] == 0x0db8)
        }
    }
}
/// Fetch a Client ID Metadata Document without following redirects, pinned to
/// a vetted public address and bounded in size.
async fn fetch_metadata(state: &AppState, client_id: &str) -> ApiResult<Value> {
    let url = reqwest::Url::parse(client_id).map_err(|_| ApiError::bad("invalid_client_id"))?;
    let host = url
        .host_str()
        .ok_or_else(|| ApiError::bad("invalid_client_id"))?
        .to_owned();
    if url.scheme() != "https"
        || url.path() == "/"
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(ApiError::bad("invalid_client_id"));
    }
    let port = url.port_or_known_default().unwrap_or(443);
    let allow_private = std::env::var("HIVE_OAUTH_ALLOW_PRIVATE_METADATA").as_deref() == Ok("1");
    let address = tokio::net::lookup_host((host.as_str(), port))
        .await
        .map_err(|_| ApiError::bad("client_metadata_unreachable"))?
        .find(|address| allow_private || public_address(address.ip()))
        .ok_or_else(|| ApiError::bad("client_metadata_address_not_public"))?;
    let timeout = std::time::Duration::from_secs(
        state
            .settings
            .runtime
            .infrastructure
            .outbound_timeout_seconds,
    );
    let client = reqwest::Client::builder()
        .resolve(&host, address)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(timeout)
        .user_agent(concat!("hiveway/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|_| ApiError::unavailable("client_metadata_unavailable"))?;
    let mut response = client
        .get(url)
        .header(header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|_| ApiError::bad("client_metadata_unreachable"))?;
    if response.status() != reqwest::StatusCode::OK {
        return Err(ApiError::bad("client_metadata_unavailable"));
    }
    let limit = state.settings.runtime.oauth.client_metadata_bytes;
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| ApiError::bad("client_metadata_unreachable"))?
    {
        body.extend_from_slice(&chunk);
        if body.len() > limit {
            return Err(ApiError::bad("client_metadata_too_large"));
        }
    }
    serde_json::from_slice(&body).map_err(|_| ApiError::bad("invalid_client_metadata"))
}

struct Client {
    id: String,
    name: String,
    uri: Option<String>,
    kind: String,
    redirect_uris: Vec<String>,
}
fn parse_redirects(value: &Value, max: usize) -> ApiResult<Vec<String>> {
    let uris: Vec<String> = value
        .as_array()
        .filter(|items| !items.is_empty() && items.len() <= max)
        .ok_or_else(|| ApiError::bad("invalid_redirect_uris"))?
        .iter()
        .map(|item| {
            item.as_str()
                .filter(|uri| valid_redirect(uri))
                .map(str::to_owned)
        })
        .collect::<Option<_>>()
        .ok_or_else(|| ApiError::bad("invalid_redirect_uri"))?;
    Ok(uris)
}
async fn store_metadata_client(state: &AppState, client_id: &str) -> ApiResult<Client> {
    let metadata = fetch_metadata(state, client_id).await?;
    let limits = &state.settings.runtime.oauth;
    if metadata["client_id"] != client_id {
        return Err(ApiError::bad("client_metadata_id_mismatch"));
    }
    if metadata["token_endpoint_auth_method"]
        .as_str()
        .is_some_and(|method| method != "none")
    {
        return Err(ApiError::bad("unsupported_client_authentication"));
    }
    let redirect_uris =
        parse_redirects(&metadata["redirect_uris"], limits.redirect_uris_per_client)?;
    let name = metadata["client_name"]
        .as_str()
        .map(|n| clean_name(n, 80))
        .filter(|n| !n.is_empty())
        .ok_or_else(|| ApiError::bad("client_name_required"))?;
    let uri = metadata["client_uri"]
        .as_str()
        .filter(|u| u.starts_with("https://") && u.len() <= 2048)
        .map(str::to_owned);
    let at = now();
    state.postgres.execute_raw(statement(
        "INSERT INTO oauth_clients(client_id,kind,client_name,client_uri,redirect_uris,application_type,created_at,last_used_at,metadata_expires_at) VALUES($1,'metadata_document',$2,$3,$4,'native',$5,$5,$6) ON CONFLICT(client_id) DO UPDATE SET client_name=EXCLUDED.client_name,client_uri=EXCLUDED.client_uri,redirect_uris=EXCLUDED.redirect_uris,metadata_expires_at=EXCLUDED.metadata_expires_at,last_used_at=EXCLUDED.last_used_at WHERE oauth_clients.kind='metadata_document'",
        vec![
            client_id.into(),
            name.clone().into(),
            uri.clone().into(),
            json!(redirect_uris).into(),
            at.into(),
            (at + limits.client_metadata_cache_seconds).into(),
        ],
    )).await?;
    Ok(Client {
        id: client_id.to_owned(),
        name,
        uri,
        kind: "metadata_document".into(),
        redirect_uris,
    })
}
/// Resolve a registered or metadata-document client.
async fn resolve_client(state: &AppState, client_id: &str) -> ApiResult<Client> {
    if client_id.is_empty() || client_id.len() > 2048 {
        return Err(ApiError::bad("invalid_client_id"));
    }
    let row = state
        .postgres
        .query_one_raw(statement(
            "SELECT client_id,kind,client_name,client_uri,redirect_uris,metadata_expires_at FROM oauth_clients WHERE client_id=$1",
            vec![client_id.into()],
        ))
        .await?;
    if let Some(row) = &row {
        let kind: String = row.try_get("", "kind")?;
        let expires: Option<i64> = row.try_get("", "metadata_expires_at")?;
        if kind == "dynamic" || expires.is_some_and(|at| at > now()) {
            let uris: Value = row.try_get("", "redirect_uris")?;
            return Ok(Client {
                id: row.try_get("", "client_id")?,
                name: row.try_get("", "client_name")?,
                uri: row.try_get("", "client_uri")?,
                kind,
                redirect_uris: uris
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|u| u.as_str().map(str::to_owned))
                    .collect(),
            });
        }
    }
    if client_id.starts_with("https://") {
        return store_metadata_client(state, client_id).await;
    }
    Err(ApiError::bad("unknown_client"))
}

/// RFC 7591 dynamic client registration for clients without metadata documents.
pub async fn register(State(state): State<AppState>, Json(request): Json<Value>) -> Response {
    let limits = &state.settings.runtime.oauth;
    let Ok(redirect_uris) =
        parse_redirects(&request["redirect_uris"], limits.redirect_uris_per_client)
    else {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_redirect_uri",
            "Use HTTPS, loopback HTTP or a private-use scheme without fragments",
        );
    };
    if request["token_endpoint_auth_method"]
        .as_str()
        .is_some_and(|method| method != "none")
    {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_client_metadata",
            "Only public clients (token_endpoint_auth_method none) are supported",
        );
    }
    if request["grant_types"].as_array().is_some_and(|grants| {
        grants
            .iter()
            .any(|g| !matches!(g.as_str(), Some("authorization_code" | "refresh_token")))
    }) {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_client_metadata",
            "Unsupported grant type",
        );
    }
    let name = request["client_name"]
        .as_str()
        .map(|n| clean_name(n, 80))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "MCP client".to_owned());
    let application_type = match request["application_type"].as_str() {
        Some("web") => "web",
        _ => "native",
    };
    let uri = request["client_uri"]
        .as_str()
        .filter(|u| u.starts_with("https://") && u.len() <= 2048)
        .map(str::to_owned);
    let client_id = format!("hwc_{}", hex::encode(rand::random::<[u8; 16]>()));
    let at = now();
    if let Err(error) = state.postgres.execute_raw(statement(
        "INSERT INTO oauth_clients(client_id,kind,client_name,client_uri,redirect_uris,application_type,created_at,last_used_at) VALUES($1,'dynamic',$2,$3,$4,$5,$6,$6)",
        vec![
            client_id.clone().into(),
            name.clone().into(),
            uri.clone().into(),
            json!(redirect_uris).into(),
            application_type.into(),
            at.into(),
        ],
    )).await {
        return *db(error);
    }
    (
        StatusCode::CREATED,
        Json(json!({
            "client_id": client_id,
            "client_id_issued_at": at,
            "client_name": name,
            "client_uri": uri,
            "redirect_uris": redirect_uris,
            "application_type": application_type,
            "grant_types": ["authorization_code", "refresh_token"],
            "response_types": ["code"],
            "token_endpoint_auth_method": "none",
        })),
    )
        .into_response()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorizeRequest {
    client_id: String,
    redirect_uri: String,
    response_type: String,
    code_challenge: String,
    code_challenge_method: String,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    resource: Option<String>,
    decision: String,
}
fn redirect_with(uri: &str, params: &[(&str, &str)]) -> ApiResult<String> {
    let mut url = reqwest::Url::parse(uri).map_err(|_| ApiError::bad("invalid_redirect_uri"))?;
    url.query_pairs_mut().extend_pairs(params.iter().copied());
    Ok(url.to_string())
}
fn granted_scopes(requested: Option<&str>, session: &Credential) -> Vec<String> {
    let held =
        |scope: &str| SESSION_SCOPES.contains(&scope) && session.scopes.iter().any(|s| s == scope);
    match requested.map(str::trim).filter(|s| !s.is_empty()) {
        Some(requested) => {
            let mut scopes: Vec<String> = requested
                .split_ascii_whitespace()
                .filter(|scope| held(scope))
                .map(str::to_owned)
                .collect();
            scopes.sort();
            scopes.dedup();
            scopes
        }
        None => SESSION_SCOPES
            .iter()
            .filter(|scope| held(scope))
            .map(|scope| (*scope).to_owned())
            .collect(),
    }
}
/// Consent decision from Nectar for the signed-in browser session.
/// `inspect` validates the request and returns what to show; `approve` and
/// `deny` return the client redirect. Invalid clients or redirect URIs never
/// produce a redirect.
/// # Errors
/// Rejects non-session credentials and malformed or unregistered requests.
pub async fn authorize(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<AuthorizeRequest>,
) -> ApiResult<Json<Value>> {
    let session = identity::authenticate_session(&state, &headers).await?;
    let client = resolve_client(&state, &request.client_id).await?;
    if !client
        .redirect_uris
        .iter()
        .any(|registered| redirect_matches(registered, &request.redirect_uri))
    {
        return Err(ApiError::bad("redirect_uri_not_registered"));
    }
    let client_state = request
        .state
        .filter(|s| s.len() <= 1024)
        .unwrap_or_default();
    let mut params: Vec<(&str, &str)> = Vec::new();
    if !client_state.is_empty() {
        params.push(("state", client_state.as_str()));
    }
    let iss = issuer(&state);
    params.push(("iss", iss.as_str()));
    let fail = |error: &'static str| -> ApiResult<Json<Value>> {
        let mut with_error = params.clone();
        with_error.push(("error", error));
        Ok(Json(
            json!({"redirect":redirect_with(&request.redirect_uri, &with_error)?}),
        ))
    };
    if request.response_type != "code" {
        return fail("unsupported_response_type");
    }
    if request.code_challenge_method != "S256"
        || !(43..=128).contains(&request.code_challenge.len())
        || !request
            .code_challenge
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return fail("invalid_request");
    }
    let expected = resource(&state);
    if request.resource.as_deref().is_some_and(|r| {
        r.trim_end_matches('/') != expected && r.trim_end_matches('/') != issuer(&state)
    }) {
        return fail("invalid_target");
    }
    let scopes = granted_scopes(request.scope.as_deref(), &session);
    if scopes.is_empty() {
        return fail("invalid_scope");
    }
    match request.decision.as_str() {
        "inspect" => Ok(Json(json!({
            "client": {
                "client_id": client.id,
                "name": client.name,
                "uri": client.uri,
                "verified_by": if client.kind == "metadata_document" { "client_metadata_document" } else { "dynamic_registration" },
                "redirect_host": reqwest::Url::parse(&request.redirect_uri).ok().and_then(|u| u.host_str().map(str::to_owned)).unwrap_or_else(|| request.redirect_uri.split(':').next().unwrap_or_default().to_owned()),
            },
            "scopes": scopes.iter().map(|s| json!({"scope":s,"description":scope_description(s)})).collect::<Vec<_>>(),
            "account": session.name,
        }))),
        "deny" => fail("access_denied"),
        "approve" => {
            let code = format!("hwo_{}", hex::encode(rand::random::<[u8; 32]>()));
            let at = now();
            state.postgres.execute_raw(statement(
                "INSERT INTO oauth_codes(code_hash,client_id,owner,redirect_uri,code_challenge,scopes,resource,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8)",
                vec![
                    identity::hash(code.as_bytes()).into(),
                    client.id.clone().into(),
                    session.wallet.clone().into(),
                    request.redirect_uri.clone().into(),
                    request.code_challenge.clone().into(),
                    scopes.into(),
                    expected.into(),
                    (at + state.settings.runtime.oauth.code_ttl_seconds).into(),
                ],
            )).await?;
            let mut with_code = params.clone();
            with_code.push(("code", code.as_str()));
            Ok(Json(
                json!({"redirect":redirect_with(&request.redirect_uri, &with_code)?}),
            ))
        }
        _ => Err(ApiError::bad("invalid_decision")),
    }
}

fn form(headers: &HeaderMap, body: &Bytes) -> OAuthResult<HashMap<String, String>> {
    let json = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("application/json"));
    let parsed = if json {
        serde_json::from_slice::<HashMap<String, Value>>(body)
            .map(|map| {
                map.into_iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_owned())))
                    .collect()
            })
            .ok()
    } else {
        serde_urlencoded::from_bytes::<Vec<(String, String)>>(body)
            .ok()
            .map(|pairs| pairs.into_iter().collect())
    };
    parsed.ok_or_else(|| {
        Box::new(oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "Malformed request body",
        ))
    })
}
fn field<'a>(params: &'a HashMap<String, String>, key: &str) -> OAuthResult<&'a str> {
    params
        .get(key)
        .map(String::as_str)
        .filter(|v| !v.is_empty() && v.len() <= 4096)
        .ok_or_else(|| {
            Box::new(oauth_error(
                StatusCode::BAD_REQUEST,
                "invalid_request",
                &format!("{key} is required"),
            ))
        })
}
#[allow(clippy::unnecessary_box_returns)]
fn invalid_grant(description: &str) -> Box<Response> {
    Box::new(oauth_error(
        StatusCode::BAD_REQUEST,
        "invalid_grant",
        description,
    ))
}
fn pkce_valid(verifier: &str, challenge: &str) -> bool {
    (43..=128).contains(&verifier.len())
        && verifier
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._~".contains(&b))
        && URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())) == challenge
}
async fn revoke_grant(txn: &impl ConnectionTrait, grant: Uuid) -> Result<(), sea_orm::DbErr> {
    let at = now();
    txn.execute_raw(statement(
        "UPDATE oauth_grants SET revoked_at=$2 WHERE id=$1 AND revoked_at IS NULL",
        vec![grant.into(), at.into()],
    ))
    .await?;
    txn.execute_raw(statement(
        "UPDATE agent_credentials SET revoked_at=$2 WHERE revoked_at IS NULL AND id IN (SELECT credential_id FROM oauth_tokens WHERE grant_id=$1)",
        vec![grant.into(), at.into()],
    ))
    .await?;
    txn.execute_raw(statement(
        "DELETE FROM oauth_refresh_tokens WHERE grant_id=$1",
        vec![grant.into()],
    ))
    .await?;
    Ok(())
}
/// Mint an access token and a rotating refresh token for a grant.
async fn issue(
    state: &AppState,
    txn: &DatabaseTransaction,
    grant: Uuid,
    owner: &str,
    client_name: &str,
    scopes: &[String],
) -> Result<Value, sea_orm::DbErr> {
    let limits = &state.settings.runtime.oauth;
    let access = format!("hwa_{}", hex::encode(rand::random::<[u8; 32]>()));
    let refresh = format!("hwr_{}", hex::encode(rand::random::<[u8; 32]>()));
    let id = Uuid::new_v4();
    let at = now();
    agent_credentials::ActiveModel {
        id: Set(id),
        wallet: Set(owner.to_owned()),
        name: Set(clean_name(&format!("OAuth: {client_name}"), 60)),
        token_hash: Set(identity::hash(access.as_bytes())),
        scopes: Set(scopes.to_vec()),
        // Free-use credentials, like account API keys: no payment budget.
        per_call_limit: Set(1),
        total_limit: Set(1),
        expires_at: Set(at + limits.access_token_ttl_seconds),
        created_at: Set(at),
        ..Default::default()
    }
    .insert(txn)
    .await?;
    txn.execute_raw(statement(
        "INSERT INTO oauth_tokens(credential_id,grant_id) VALUES($1,$2)",
        vec![id.into(), grant.into()],
    ))
    .await?;
    txn.execute_raw(statement(
        "INSERT INTO oauth_refresh_tokens(token_hash,grant_id,expires_at) VALUES($1,$2,$3)",
        vec![
            identity::hash(refresh.as_bytes()).into(),
            grant.into(),
            (at + limits.refresh_token_ttl_seconds).into(),
        ],
    ))
    .await?;
    txn.execute_raw(statement(
        "UPDATE oauth_grants SET last_used_at=$2 WHERE id=$1",
        vec![grant.into(), at.into()],
    ))
    .await?;
    Ok(json!({
        "access_token": access,
        "token_type": "Bearer",
        "expires_in": limits.access_token_ttl_seconds,
        "refresh_token": refresh,
        "scope": scopes.join(" "),
    }))
}
async fn exchange_code(state: &AppState, params: &HashMap<String, String>) -> OAuthResult<Value> {
    let code = field(params, "code")?;
    let client_id = field(params, "client_id")?;
    let redirect_uri = field(params, "redirect_uri")?;
    let verifier = field(params, "code_verifier")?;
    let txn = state.postgres.begin().await.map_err(db)?;
    let row = txn
        .query_one_raw(statement(
            "SELECT c.client_id,c.owner,c.redirect_uri,c.code_challenge,c.scopes,c.resource,c.expires_at,c.used_at,c.grant_id,k.client_name FROM oauth_codes c JOIN oauth_clients k ON k.client_id=c.client_id WHERE c.code_hash=$1 FOR UPDATE OF c",
            vec![identity::hash(code.as_bytes()).into()],
        ))
        .await
        .map_err(db)?
        .ok_or_else(|| invalid_grant("Unknown authorization code"))?;
    let get = |key: &str| row.try_get::<String>("", key).map_err(db);
    if row
        .try_get::<Option<i64>>("", "used_at")
        .map_err(db)?
        .is_some()
    {
        // Code replay: revoke whatever the first redemption produced.
        if let Some(grant) = row.try_get::<Option<Uuid>>("", "grant_id").map_err(db)? {
            revoke_grant(&txn, grant).await.map_err(db)?;
            txn.commit().await.map_err(db)?;
        }
        return Err(invalid_grant("Authorization code already used"));
    }
    if row.try_get::<i64>("", "expires_at").map_err(db)? <= now()
        || get("client_id")? != client_id
        || get("redirect_uri")? != redirect_uri
        || !pkce_valid(verifier, &get("code_challenge")?)
        || params
            .get("resource")
            .is_some_and(|r| r.trim_end_matches('/') != get("resource").unwrap_or_default())
    {
        return Err(invalid_grant(
            "Authorization code is invalid for this request",
        ));
    }
    let owner = get("owner")?;
    let scopes: Vec<String> = row.try_get("", "scopes").map_err(db)?;
    let grant = Uuid::new_v4();
    let at = now();
    txn.execute_raw(statement(
        "INSERT INTO oauth_grants(id,client_id,owner,scopes,created_at,last_used_at) VALUES($1,$2,$3,$4,$5,$5)",
        vec![grant.into(), client_id.into(), owner.clone().into(), scopes.clone().into(), at.into()],
    ))
    .await
    .map_err(db)?;
    txn.execute_raw(statement(
        "UPDATE oauth_codes SET used_at=$2, grant_id=$3 WHERE code_hash=$1",
        vec![
            identity::hash(code.as_bytes()).into(),
            at.into(),
            grant.into(),
        ],
    ))
    .await
    .map_err(db)?;
    let tokens = issue(state, &txn, grant, &owner, &get("client_name")?, &scopes)
        .await
        .map_err(db)?;
    txn.commit().await.map_err(db)?;
    Ok(tokens)
}
async fn refresh(state: &AppState, params: &HashMap<String, String>) -> OAuthResult<Value> {
    let token = field(params, "refresh_token")?;
    let client_id = field(params, "client_id")?;
    let txn = state.postgres.begin().await.map_err(db)?;
    let row = txn
        .query_one_raw(statement(
            "SELECT r.grant_id,r.expires_at,r.used_at,g.client_id,g.owner,g.scopes,g.revoked_at,k.client_name FROM oauth_refresh_tokens r JOIN oauth_grants g ON g.id=r.grant_id JOIN oauth_clients k ON k.client_id=g.client_id WHERE r.token_hash=$1 FOR UPDATE OF r, g",
            vec![identity::hash(token.as_bytes()).into()],
        ))
        .await
        .map_err(db)?
        .ok_or_else(|| invalid_grant("Unknown refresh token"))?;
    let grant: Uuid = row.try_get("", "grant_id").map_err(db)?;
    if row
        .try_get::<Option<i64>>("", "used_at")
        .map_err(db)?
        .is_some()
    {
        // A rotated token came back: assume theft and end the whole grant.
        revoke_grant(&txn, grant).await.map_err(db)?;
        txn.commit().await.map_err(db)?;
        return Err(invalid_grant(
            "Refresh token reuse detected; the grant was revoked",
        ));
    }
    if row.try_get::<i64>("", "expires_at").map_err(db)? <= now()
        || row
            .try_get::<Option<i64>>("", "revoked_at")
            .map_err(db)?
            .is_some()
        || row.try_get::<String>("", "client_id").map_err(db)? != client_id
    {
        return Err(invalid_grant("Refresh token is invalid for this client"));
    }
    let held: Vec<String> = row.try_get("", "scopes").map_err(db)?;
    let scopes: Vec<String> = match params.get("scope").map(String::as_str) {
        Some(requested) if !requested.trim().is_empty() => {
            let narrowed: Vec<String> = requested
                .split_ascii_whitespace()
                .filter(|s| held.iter().any(|h| h == s))
                .map(str::to_owned)
                .collect();
            if narrowed.is_empty() {
                return Err(Box::new(oauth_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_scope",
                    "Requested scope exceeds the grant",
                )));
            }
            narrowed
        }
        _ => held,
    };
    txn.execute_raw(statement(
        "UPDATE oauth_refresh_tokens SET used_at=$2 WHERE token_hash=$1",
        vec![identity::hash(token.as_bytes()).into(), now().into()],
    ))
    .await
    .map_err(db)?;
    let owner: String = row.try_get("", "owner").map_err(db)?;
    let name: String = row.try_get("", "client_name").map_err(db)?;
    let tokens = issue(state, &txn, grant, &owner, &name, &scopes)
        .await
        .map_err(db)?;
    txn.commit().await.map_err(db)?;
    Ok(tokens)
}
/// Token endpoint: `authorization_code` and `refresh_token` grants.
pub async fn token(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    let params = match form(&headers, &body) {
        Ok(params) => params,
        Err(response) => return *response,
    };
    let result = match params.get("grant_type").map(String::as_str) {
        Some("authorization_code") => exchange_code(&state, &params).await,
        Some("refresh_token") => refresh(&state, &params).await,
        _ => Err(Box::new(oauth_error(
            StatusCode::BAD_REQUEST,
            "unsupported_grant_type",
            "Use authorization_code or refresh_token",
        ))),
    };
    match result {
        Ok(tokens) => {
            let mut response = Json(tokens).into_response();
            response
                .headers_mut()
                .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
            response
        }
        Err(response) => *response,
    }
}
/// RFC 7009 revocation. Unknown tokens are not an error.
pub async fn revoke(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    let params = match form(&headers, &body) {
        Ok(params) => params,
        Err(response) => return *response,
    };
    let Ok(token) = field(&params, "token") else {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "token is required",
        );
    };
    let hash = identity::hash(token.as_bytes());
    let sql = if token.starts_with("hwr_") {
        "SELECT r.grant_id, g.client_id FROM oauth_refresh_tokens r JOIN oauth_grants g ON g.id=r.grant_id WHERE r.token_hash=$1"
    } else {
        "SELECT t.grant_id, g.client_id FROM agent_credentials a JOIN oauth_tokens t ON t.credential_id=a.id JOIN oauth_grants g ON g.id=t.grant_id WHERE a.token_hash=$1"
    };
    let outcome = async {
        let Some(row) = state
            .postgres
            .query_one_raw(statement(sql, vec![hash.into()]))
            .await?
        else {
            return Ok(());
        };
        let client: String = row.try_get("", "client_id")?;
        if params.get("client_id").is_some_and(|id| *id != client) {
            return Ok(());
        }
        let grant: Uuid = row.try_get("", "grant_id")?;
        let txn = state.postgres.begin().await?;
        revoke_grant(&txn, grant).await?;
        txn.commit().await
    }
    .await;
    match outcome {
        Ok(()) => StatusCode::OK.into_response(),
        Err(error) => *db(error),
    }
}
/// Connected OAuth applications for the signed-in account.
/// # Errors
/// Rejects non-session credentials.
pub async fn grants(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let session = identity::authenticate_session(&state, &headers).await?;
    let rows = state
        .postgres
        .query_all_raw(statement(
            "SELECT g.id,g.scopes,g.created_at,g.last_used_at,k.client_name,k.client_id,k.client_uri,k.kind FROM oauth_grants g JOIN oauth_clients k ON k.client_id=g.client_id WHERE g.owner=$1 AND g.revoked_at IS NULL AND EXISTS(SELECT 1 FROM oauth_refresh_tokens r WHERE r.grant_id=g.id AND r.used_at IS NULL AND r.expires_at>$2) ORDER BY g.last_used_at DESC LIMIT 200",
            vec![session.wallet.into(), now().into()],
        ))
        .await?;
    let grants = rows
        .iter()
        .map(|row| {
            Ok(json!({
                "id": row.try_get::<Uuid>("", "id")?,
                "client_name": row.try_get::<String>("", "client_name")?,
                "client_id": row.try_get::<String>("", "client_id")?,
                "client_uri": row.try_get::<Option<String>>("", "client_uri")?,
                "verified_by": row.try_get::<String>("", "kind")?,
                "scopes": row.try_get::<Vec<String>>("", "scopes")?,
                "created_at": row.try_get::<i64>("", "created_at")?,
                "last_used_at": row.try_get::<i64>("", "last_used_at")?,
            }))
        })
        .collect::<ApiResult<Vec<_>>>()?;
    Ok(Json(json!({"grants":grants})))
}
/// Disconnect an OAuth application immediately.
/// # Errors
/// Rejects foreign grants and non-session credentials.
pub async fn revoke_owned_grant(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    let session = identity::authenticate_session(&state, &headers).await?;
    let txn = state.postgres.begin().await?;
    let owned = txn
        .query_one_raw(statement(
            "SELECT id FROM oauth_grants WHERE id=$1 AND owner=$2 FOR UPDATE",
            vec![id.into(), session.wallet.into()],
        ))
        .await?;
    if owned.is_none() {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "grant_not_found"));
    }
    revoke_grant(&txn, id).await?;
    txn.commit().await?;
    Ok(Json(json!({"revoked":true})))
}
/// Remove expired codes and tokens, and registrations nobody uses.
/// # Errors
/// Returns database errors.
pub async fn cleanup(state: &AppState) -> ApiResult<()> {
    let at = now();
    let retention = state.settings.runtime.oauth.unused_client_retention_seconds;
    for (sql, values) in [
        (
            "DELETE FROM oauth_codes WHERE expires_at<$1",
            vec![at.into()],
        ),
        (
            "DELETE FROM oauth_refresh_tokens WHERE expires_at<$1 OR (used_at IS NOT NULL AND used_at<$2)",
            vec![at.into(), (at - 86_400).into()],
        ),
        (
            "DELETE FROM oauth_grants WHERE revoked_at IS NOT NULL AND revoked_at<$1",
            vec![(at - retention).into()],
        ),
        (
            "DELETE FROM oauth_clients c WHERE c.last_used_at<$1 AND NOT EXISTS(SELECT 1 FROM oauth_grants g WHERE g.client_id=c.client_id)",
            vec![(at - retention).into()],
        ),
    ] {
        state.postgres.execute_raw(statement(sql, values)).await?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn redirect_uris_follow_native_app_rules() {
        for ok in [
            "https://claude.ai/api/mcp/auth_callback",
            "http://127.0.0.1:33418/callback",
            "http://localhost/callback",
            "cursor://anysphere.cursor-retrieval/oauth/callback",
            "com.example.app:/oauth",
        ] {
            assert!(valid_redirect(ok), "{ok}");
        }
        for bad in [
            "http://example.com/callback",
            "javascript:alert(1)",
            "data:text/html,x",
            "file:///etc/passwd",
            "https://user:pw@example.com/cb",
            "https://example.com/cb#fragment",
            "not a url",
        ] {
            assert!(!valid_redirect(bad), "{bad}");
        }
    }
    #[test]
    fn loopback_redirects_may_change_port_only() {
        assert!(redirect_matches(
            "http://127.0.0.1/callback",
            "http://127.0.0.1:5173/callback"
        ));
        assert!(!redirect_matches(
            "http://127.0.0.1/callback",
            "http://127.0.0.1:5173/other"
        ));
        assert!(!redirect_matches(
            "https://a.example/cb",
            "https://a.example:444/cb"
        ));
        assert!(redirect_matches(
            "https://a.example/cb",
            "https://a.example/cb"
        ));
    }
    #[test]
    fn pkce_requires_the_s256_verifier() {
        let verifier = "a".repeat(43);
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        assert!(pkce_valid(&verifier, &challenge));
        assert!(!pkce_valid(&"b".repeat(43), &challenge));
        assert!(!pkce_valid("short", &challenge));
    }
    #[test]
    fn metadata_fetches_only_reach_public_addresses() -> anyhow::Result<()> {
        for private in [
            "127.0.0.1",
            "10.1.2.3",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "::1",
            "fd00::1",
            "fe80::1",
            "::ffff:10.0.0.1",
            "0.0.0.0",
        ] {
            assert!(!public_address(private.parse()?), "{private}");
        }
        for public in ["1.1.1.1", "140.82.112.3", "2606:4700::1111"] {
            assert!(public_address(public.parse()?), "{public}");
        }
        Ok(())
    }
    #[test]
    fn requested_scopes_never_exceed_the_session() {
        let session = Credential {
            id: Uuid::nil(),
            wallet: "github:1".into(),
            name: "octo".into(),
            scopes: vec!["capabilities:read".into(), "memory:read".into()],
            per_call_limit: 1,
            total_limit: 1,
            reserved: 0,
            spent: 0,
            expires_at: i64::MAX,
            revoked_at: None,
        };
        assert_eq!(
            granted_scopes(None, &session),
            vec!["capabilities:read", "memory:read"]
        );
        assert_eq!(
            granted_scopes(Some("memory:read sales:read memory:read"), &session),
            vec!["memory:read"]
        );
        assert_eq!(
            granted_scopes(Some("capabilities:publish"), &session),
            Vec::<String>::new()
        );
    }
}
