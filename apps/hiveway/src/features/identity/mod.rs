pub mod signature;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use sea_orm::{ConnectionTrait, TransactionTrait};
use std::net::SocketAddr;

use axum::{
    Json,
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
};
use rand::random;
use sha2::{Digest, Sha256};

use uuid::Uuid;

use crate::{
    features::common::{
        error::{ApiError, ApiResult},
        model::{
            AuthResponse, ChallengeRequest, ChallengeResponse, Credential, CredentialPolicy,
            CredentialStatus, SCOPES, VerifyRequest, WalletAction, now,
        },
    },
    state::AppState,
};

/// # Errors
/// Rejects missing, malformed, or out-of-range atomic amounts.
pub fn amount(value: &str) -> ApiResult<i64> {
    if value.is_empty() || value.len() > 10 || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ApiError::bad("invalid_amount"));
    }
    value
        .parse::<i64>()
        .ok()
        .filter(|n| (1..=1_000_000_000).contains(n))
        .ok_or_else(|| ApiError::bad("invalid_amount"))
}
/// # Errors
/// Rejects malformed addresses and the zero address.
pub fn wallet(value: &str) -> ApiResult<String> {
    let bytes = hex::decode(
        value
            .strip_prefix("0x")
            .ok_or_else(|| ApiError::bad("invalid_wallet"))?,
    )
    .map_err(|_| ApiError::bad("invalid_wallet"))?;
    if bytes.len() != 20 || bytes.iter().all(|b| *b == 0) {
        return Err(ApiError::bad("invalid_wallet"));
    }
    Ok(format!("0x{}", hex::encode(bytes)))
}
#[must_use]
pub fn hash(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}

/// # Errors
/// Rejects unsupported scopes, invalid names, and unsafe budget or expiry limits.
pub fn validate_policy(p: &CredentialPolicy, at: i64) -> ApiResult<()> {
    if p.name.is_empty()
        || p.name.len() > 80
        || !p
            .name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b' ' || b == b'-' || b == b'_')
    {
        return Err(ApiError::bad("invalid_agent_name"));
    }
    if p.scopes.is_empty()
        || p.scopes.len() > SCOPES.len()
        || p.scopes.iter().any(|s| !SCOPES.contains(&s.as_str()))
    {
        return Err(ApiError::bad("invalid_scopes"));
    }
    let mut scopes = p.scopes.clone();
    scopes.sort();
    scopes.dedup();
    if scopes.len() != p.scopes.len() {
        return Err(ApiError::bad("duplicate_scopes"));
    }
    if amount(&p.per_call_limit)? > amount(&p.total_limit)?
        || p.expires_at <= at
        || p.expires_at > at + 86400
    {
        return Err(ApiError::bad("invalid_credential_limits"));
    }
    Ok(())
}

#[utoipa::path(post, path="/v1/auth/challenges", request_body=ChallengeRequest, responses((status=200,body=ChallengeResponse)))]
/// # Errors
/// Rejects invalid policies or addresses, rate-limited peers, and database failures.
pub async fn challenge(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(request): Json<ChallengeRequest>,
) -> ApiResult<Json<ChallengeResponse>> {
    let at = now();
    let address = wallet(&request.wallet)?;
    if let WalletAction::Issue { policy } = &request.action {
        validate_policy(policy, at)?;
    }
    let tx = state.postgres.begin().await?;
    // Serialize challenge issuance per socket peer so concurrent requests cannot bypass the limit.
    let ip = peer.ip().to_string();
    tx.execute_raw(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        "SELECT pg_advisory_xact_lock(hashtextextended($1,0))",
        vec![(&ip).into()],
    ))
    .await?;
    tx.execute_raw(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        "SELECT pg_advisory_xact_lock(hashtextextended('hive:wallet:' || $1,0))",
        [address.clone().into()],
    ))
    .await?;
    let count: i64=tx.query_one_raw(sea_orm::Statement::from_sql_and_values(sea_orm::DbBackend::Postgres, "SELECT count(*) FROM wallet_challenges WHERE (wallet=$1 OR peer_ip=$2) AND created_at>$3", vec![(&address).into(), (&ip).into(), (at-60).into()])).await?.ok_or_else(|| sea_orm::DbErr::RecordNotFound("required row missing".into()))?.try_get_by_index(0)?;
    if count >= 10 {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "challenge_rate_limit",
        ));
    }
    let id = Uuid::new_v4();
    let expires = at + 300;
    let policy_hash = hex::encode(hash(&serde_json::to_vec(&request.action)?));
    let statement = match &request.action {
        WalletAction::Issue { policy } => format!(
            "Issue Hiveway credential for {}. Scopes: {}. Per-call limit: {} atomic payment units. Total limit: {} atomic payment units. Credential expires: {}. Policy SHA256: {}.",
            policy.name,
            policy.scopes.join(","),
            policy.per_call_limit,
            policy.total_limit,
            policy.expires_at,
            policy_hash
        ),
        WalletAction::Revoke { credential_id } => {
            format!("Revoke Hiveway credential {credential_id}. Policy SHA256: {policy_hash}.")
        }
    };
    let message = signature::message(&state.settings, &address, &statement, id, at, expires)?;
    tx.execute_raw(sea_orm::Statement::from_sql_and_values(sea_orm::DbBackend::Postgres, "INSERT INTO wallet_challenges(id,wallet,peer_ip,message,action,expires_at,created_at) VALUES($1,$2,$3,$4,$5,$6,$7)", vec![(id).into(), (address).into(), (ip).into(), (&message).into(), (serde_json::to_value(request.action)?).into(), (expires).into(), (at).into()])).await?;
    tx.commit().await?;
    Ok(Json(ChallengeResponse {
        id,
        message,
        expires_at: expires,
    }))
}

#[utoipa::path(post,path="/v1/auth/verify",request_body=VerifyRequest,responses((status=200,body=AuthResponse)))]
/// # Errors
/// Rejects invalid, expired, or replayed signatures and propagates database failures.
pub async fn verify(
    State(state): State<AppState>,
    Json(request): Json<VerifyRequest>,
) -> ApiResult<Json<AuthResponse>> {
    let tx = state.postgres.begin().await?;
    let at = now();
    let row=tx.query_one_raw(sea_orm::Statement::from_sql_and_values(sea_orm::DbBackend::Postgres, "SELECT wallet,message,action,expires_at,consumed_at FROM wallet_challenges WHERE id=$1 FOR UPDATE", vec![(request.challenge_id).into()])).await?.ok_or_else(||ApiError::bad("unknown_challenge"))?;
    if row.try_get::<Option<i64>>("", "consumed_at")?.is_some()
        || row.try_get::<i64>("", "expires_at")? <= at
    {
        return Err(ApiError::bad("challenge_expired_or_used"));
    }
    let text: String = row.try_get("", "message")?;
    let owner: String = row.try_get("", "wallet")?;
    let recovered = signature::recover(&text, &request.signature)?;
    if recovered != owner {
        return Err(ApiError::new(StatusCode::UNAUTHORIZED, "invalid_signature"));
    }
    let action: WalletAction = serde_json::from_value(row.try_get("", "action")?)?;
    let response = match action {
        WalletAction::Issue { policy } => {
            validate_policy(&policy, at)?;
            let token = format!("hwa_{}", hex::encode(random::<[u8; 32]>()));
            let id = Uuid::new_v4();
            tx.execute_raw(sea_orm::Statement::from_sql_and_values(sea_orm::DbBackend::Postgres, "INSERT INTO agent_credentials(id,wallet,name,token_hash,scopes,per_call_limit,total_limit,expires_at,created_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)", vec![(id).into(), (owner).into(), (policy.name).into(), (hash(token.as_bytes())).into(), (policy.scopes).into(), (amount(&policy.per_call_limit)?).into(), (amount(&policy.total_limit)?).into(), (policy.expires_at).into(), (at).into()])).await?;
            AuthResponse {
                credential_id: id,
                token: Some(token),
                revoked: false,
            }
        }
        WalletAction::Revoke { credential_id } => {
            let count = tx
                .execute_raw(sea_orm::Statement::from_sql_and_values(
                    sea_orm::DbBackend::Postgres,
                    "UPDATE agent_credentials SET revoked_at=$1 WHERE id=$2 AND wallet=$3",
                    vec![(at).into(), (credential_id).into(), (owner).into()],
                ))
                .await?
                .rows_affected();
            if count != 1 {
                return Err(ApiError::new(StatusCode::NOT_FOUND, "credential_not_found"));
            }
            AuthResponse {
                credential_id,
                token: None,
                revoked: true,
            }
        }
    };
    tx.execute_raw(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        "UPDATE wallet_challenges SET consumed_at=$1 WHERE id=$2",
        vec![(at).into(), (request.challenge_id).into()],
    ))
    .await?;
    tx.commit().await?;
    Ok(Json(response))
}

/// # Errors
/// Rejects missing, expired, revoked, or insufficiently scoped credentials and database failures.
pub async fn authenticate(
    state: &AppState,
    headers: &HeaderMap,
    scope: &str,
) -> ApiResult<Credential> {
    if headers.get_all("authorization").iter().count() != 1 {
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "credential_required",
        ));
    }
    let raw = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .filter(|v| v.len() == 68 && v.starts_with("hwa_"))
        .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "credential_required"))?;
    let credential = crate::entities::agent_credentials::Entity::find()
        .filter(crate::entities::agent_credentials::Column::TokenHash.eq(hash(raw.as_bytes())))
        .into_model::<Credential>()
        .one(&state.postgres)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "invalid_credential"))?;
    authorize(&credential, scope)?;
    Ok(credential)
}
/// # Errors
/// Rejects expired, revoked, or insufficiently scoped credentials.
pub fn authorize(credential: &Credential, scope: &str) -> ApiResult<()> {
    if credential.revoked_at.is_some() || credential.expires_at <= now() {
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "credential_expired_or_revoked",
        ));
    }
    if !scope.is_empty() && !credential.scopes.iter().any(|s| s == scope) {
        return Err(ApiError::forbidden());
    }
    Ok(())
}
#[utoipa::path(get,path="/v1/credentials/me",responses((status=200,body=CredentialStatus)))]
/// # Errors
/// Rejects invalid credentials and propagates database failures.
pub async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<CredentialStatus>> {
    let c = authenticate(&state, &headers, "").await?;
    Ok(Json(c.into()))
}
