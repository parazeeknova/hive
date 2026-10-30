//! Quote admission and budget reservations, independent from invocation settlement.
mod replay;
mod reservations;
use super::{Tx, locked_credential, required, x402};
use crate::{
    entities::{
        agent_credentials::{Column as AgentCredentialsColumn, Entity as AgentCredentialsEntity},
        capabilities::{Column as CapabilitiesColumn, Entity as CapabilitiesEntity},
        invocations::{Column as InvocationsColumn, Entity as InvocationsEntity},
    },
    features::{
        cell,
        common::{
            error::{ApiError, ApiResult},
            model::{
                Capability, Invocation, PaymentRequired, PaymentRequirements, QuoteRequest,
                QuoteResponse, Resource, now,
            },
        },
        identity,
    },
    state::AppState,
};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use replay::existing_quote;
use reservations::{release_expired, reserve};
use sea_orm::sea_query::{Alias, Expr, ExprTrait, Query as SqlQuery};
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, TransactionTrait,
};
use serde_json::json;
use uuid::Uuid;
fn idempotency_key(headers: &HeaderMap) -> ApiResult<Uuid> {
    if headers.get_all("idempotency-key").iter().count() != 1 {
        return Err(ApiError::bad("uuid_idempotency_key_required"));
    }
    headers
        .get("idempotency-key")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| ApiError::bad("uuid_idempotency_key_required"))
}

#[utoipa::path(post,path="/v1/quotes",request_body=QuoteRequest,responses((status=201,body=QuoteResponse)))]
/// # Errors
/// Rejects invalid credentials or input, conflicting idempotency keys, insufficient budgets, and unavailable payment infrastructure.
pub async fn quote(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<QuoteRequest>,
) -> ApiResult<Response> {
    let c = identity::authenticate_invoker(&state, &headers).await?;
    let key = idempotency_key(&headers)?;
    let fingerprint = identity::hash(&serde_json::to_vec(&request)?);
    if let Some(result) = existing_quote(&state.postgres, c.id, key, &fingerprint).await? {
        return Ok((StatusCode::OK, Json(result)).into_response());
    }
    cell::validate_input(&state, request.capability_id, &request.input).await?;
    // Do remote I/O before acquiring a credential row lock or a pool transaction.
    let price = CapabilitiesEntity::find_by_id(request.capability_id)
        .one(&state.postgres)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "capability_not_found"))?
        .price;
    if price > 0 {
        x402::supported(&state).await?;
    }
    let tx = state.postgres.begin().await?;
    let c = locked_credential(&tx, c.id, identity::invocation_scope(&c)?).await?;
    if let Some(result) = existing_quote(&tx, c.id, key, &fingerprint).await? {
        tx.commit().await?;
        return Ok((StatusCode::OK, Json(result)).into_response());
    }
    release_expired(&tx, c.id).await?;
    let cap = CapabilitiesEntity::find()
        .filter(
            Expr::col(CapabilitiesColumn::Id)
                .eq(Expr::val(request.capability_id))
                .and(Expr::col(CapabilitiesColumn::Active)),
        )
        .lock_shared()
        .into_model::<Capability>()
        .one(&tx)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "capability_not_found"))?;
    if cap.runtime == "wasm" && identity::invocation_scope(&c)? != "capabilities:invoke" {
        return Err(ApiError::forbidden());
    }
    reserve(&tx, c.id, cap.price).await?;
    let id = Uuid::new_v4();
    let at = now();
    let expires_at = at + 120;
    let requirements = PaymentRequirements {
        scheme: "exact".into(),
        network: state.settings.runtime.payment.network(),
        amount: cap.price.to_string(),
        asset: state.settings.runtime.payment.asset.clone(),
        pay_to: cap.creator_wallet,
        max_timeout_seconds: 120,
        extra: json!({"name":state.settings.runtime.payment.name,"version":state.settings.runtime.payment.version}),
    };
    let resource = Resource {
        url: format!("{}/v1/invocations/{id}", state.settings.public_url),
        description: cap.description,
        mime_type: "application/json".into(),
    };
    crate::entities::invocations::ActiveModel {
        id: sea_orm::Set(id),
        credential_id: sea_orm::Set(c.id),
        capability_id: sea_orm::Set(cap.id),
        idempotency_key: sea_orm::Set(key),
        request_hash: sea_orm::Set(fingerprint),
        input: sea_orm::Set(serde_json::to_value(request.input)?),
        requirements: sea_orm::Set(serde_json::to_value(&requirements)?),
        resource: sea_orm::Set(serde_json::to_value(&resource)?),
        state: sea_orm::Set("quoted".into()),
        price: sea_orm::Set(cap.price),
        expires_at: sea_orm::Set(expires_at),
        created_at: sea_orm::Set(at),
        updated_at: sea_orm::Set(at),
        creator_wallet: sea_orm::Set(requirements.pay_to.clone()),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(QuoteResponse {
            id,
            expires_at,
            payment: PaymentRequired {
                x402_version: 2,
                resource,
                accepts: if cap.price == 0 {
                    Vec::new()
                } else {
                    vec![requirements]
                },
            },
        }),
    )
        .into_response())
}
