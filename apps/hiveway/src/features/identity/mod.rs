use crate::entities::agent_credentials::Column as AgentCredentialsColumn;
use crate::entities::agent_credentials::Entity as AgentCredentialsEntity;
use crate::entities::wallet_challenges::Column as WalletChallengesColumn;
use crate::entities::wallet_challenges::Entity as WalletChallengesEntity;
use sea_orm::sea_query::{Alias, Asterisk, Expr, ExprTrait, Func, Query as SqlQuery};
use sea_orm::{ActiveModelTrait, QuerySelect};
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
    for resource in [format!("hive:ip:{ip}"), format!("hive:wallet:{address}")] {
        use sha2::Digest;
        let digest = sha2::Sha256::digest(resource.as_bytes());
        let key = i64::from_be_bytes(
            digest[..8]
                .try_into()
                .map_err(|_| ApiError::bad("invalid_lock_key"))?,
        );
        tx.query_one(
            &SqlQuery::select()
                .expr(Func::cust(Alias::new("pg_advisory_xact_lock")).arg(key))
                .to_owned(),
        )
        .await?;
    }
    let count: i64 = tx
        .query_one(
            &SqlQuery::select()
                .from(WalletChallengesEntity)
                .expr(Expr::col(Asterisk).count())
                .and_where(
                    Expr::col(WalletChallengesColumn::Wallet)
                        .eq(Expr::val(address.clone()))
                        .or(Expr::col(WalletChallengesColumn::PeerIp).eq(Expr::val(ip.clone())))
                        .and(Expr::col(WalletChallengesColumn::CreatedAt).gt(Expr::val(at - 60))),
                )
                .to_owned(),
        )
        .await?
        .ok_or_else(|| sea_orm::DbErr::RecordNotFound("required row missing".into()))?
        .try_get_by_index(0)?;
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
    tx.execute(
        &SqlQuery::insert()
            .into_table(WalletChallengesEntity)
            .columns([
                WalletChallengesColumn::Id,
                WalletChallengesColumn::Wallet,
                WalletChallengesColumn::PeerIp,
                WalletChallengesColumn::Message,
                WalletChallengesColumn::Action,
                WalletChallengesColumn::ExpiresAt,
                WalletChallengesColumn::CreatedAt,
            ])
            .values_panic([
                Expr::val(id),
                Expr::val((address).clone()),
                Expr::val((ip).clone()),
                Expr::val(message.clone()),
                Expr::val((serde_json::to_value(request.action)?).clone()),
                Expr::val(expires),
                Expr::val(at),
            ])
            .to_owned(),
    )
    .await?;
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
    let row = WalletChallengesEntity::find_by_id(request.challenge_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| ApiError::bad("unknown_challenge"))?;
    if row.consumed_at.is_some() || row.expires_at <= at {
        return Err(ApiError::bad("challenge_expired_or_used"));
    }
    let text: String = row.message.clone();
    let owner: String = row.wallet.clone();
    let recovered = signature::recover(&text, &request.signature)?;
    if recovered != owner {
        return Err(ApiError::new(StatusCode::UNAUTHORIZED, "invalid_signature"));
    }
    let action: WalletAction = serde_json::from_value(row.action.clone())?;
    let response = match action {
        WalletAction::Issue { policy } => {
            validate_policy(&policy, at)?;
            let token = format!("hwa_{}", hex::encode(random::<[u8; 32]>()));
            let id = Uuid::new_v4();
            crate::entities::agent_credentials::ActiveModel {
                id: sea_orm::Set(id),
                wallet: sea_orm::Set(owner),
                name: sea_orm::Set(policy.name),
                token_hash: sea_orm::Set(hash(token.as_bytes())),
                scopes: sea_orm::Set(policy.scopes),
                per_call_limit: sea_orm::Set(amount(&policy.per_call_limit)?),
                total_limit: sea_orm::Set(amount(&policy.total_limit)?),
                expires_at: sea_orm::Set(policy.expires_at),
                created_at: sea_orm::Set(at),
                ..Default::default()
            }
            .insert(&tx)
            .await?;
            AuthResponse {
                credential_id: id,
                token: Some(token),
                revoked: false,
            }
        }
        WalletAction::Revoke { credential_id } => {
            let count = AgentCredentialsEntity::update_many()
                .col_expr(AgentCredentialsColumn::RevokedAt, Expr::val(at))
                .filter(
                    Expr::col(AgentCredentialsColumn::Id)
                        .eq(Expr::val(credential_id))
                        .and(
                            Expr::col(AgentCredentialsColumn::Wallet)
                                .eq(Expr::val((owner).clone())),
                        ),
                )
                .exec(&tx)
                .await?
                .rows_affected;
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
    WalletChallengesEntity::update_many()
        .col_expr(WalletChallengesColumn::ConsumedAt, Expr::val(at))
        .filter(Expr::col(WalletChallengesColumn::Id).eq(Expr::val(request.challenge_id)))
        .exec(&tx)
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
    let credential = AgentCredentialsEntity::find()
        .filter(AgentCredentialsColumn::TokenHash.eq(hash(raw.as_bytes())))
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
