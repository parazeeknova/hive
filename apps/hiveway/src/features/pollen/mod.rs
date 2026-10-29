use crate::entities::agent_credentials::Column as AgentCredentialsColumn;
use crate::entities::agent_credentials::Entity as AgentCredentialsEntity;
use crate::entities::creator_sales::Column as CreatorSalesColumn;
use crate::entities::creator_sales::Entity as CreatorSalesEntity;
use crate::entities::invocations::Column as InvocationsColumn;
use crate::entities::invocations::Entity as InvocationsEntity;
use sea_orm::sea_query::{Expr, ExprTrait, Query as SqlQuery};
use sea_orm::{ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, TransactionTrait};
pub mod dodo;
pub mod quotes;
pub mod worker;
pub mod x402;
pub use quotes::quote;

use crate::{
    features::{
        cell,
        common::{
            error::{ApiError, ApiResult},
            model::{
                Credential, Invocation, PaymentPayload, PaymentRequired, PaymentRequirements,
                Receipt, Settlement, now,
            },
        },
        identity,
    },
    state::AppState,
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use sea_orm::DatabaseTransaction;
use uuid::Uuid;

type Tx = DatabaseTransaction;
pub(super) async fn locked_credential(tx: &Tx, id: Uuid, scope: &str) -> ApiResult<Credential> {
    let c = AgentCredentialsEntity::find()
        .filter(Expr::col(AgentCredentialsColumn::Id).eq(Expr::val(id)))
        .lock_exclusive()
        .into_model::<Credential>()
        .one(tx)
        .await?
        .ok_or_else(|| sea_orm::DbErr::RecordNotFound("required row missing".into()))?;
    identity::authorize(&c, scope)?;
    Ok(c)
}
pub(super) async fn invocation(tx: &Tx, id: Uuid, credential: Uuid) -> ApiResult<Invocation> {
    InvocationsEntity::find()
        .filter(
            Expr::col(InvocationsColumn::Id)
                .eq(Expr::val(id))
                .and(Expr::col(InvocationsColumn::CredentialId).eq(Expr::val(credential))),
        )
        .lock_exclusive()
        .into_model::<Invocation>()
        .one(tx)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "invocation_not_found"))
}
async fn release(tx: &Tx, inv: &Invocation, state: &str, error: &str) -> ApiResult<()> {
    InvocationsEntity::update_many()
        .col_expr(InvocationsColumn::State, Expr::val(state))
        .col_expr(InvocationsColumn::ErrorCode, Expr::val(error))
        .col_expr(InvocationsColumn::UpdatedAt, Expr::val(now()))
        .filter(Expr::col(InvocationsColumn::Id).eq(Expr::val(inv.id)))
        .exec(tx)
        .await?;
    AgentCredentialsEntity::update_many()
        .col_expr(
            AgentCredentialsColumn::Reserved,
            Expr::col(AgentCredentialsColumn::Reserved).sub(Expr::val(inv.price)),
        )
        .filter(Expr::col(AgentCredentialsColumn::Id).eq(Expr::val(inv.credential_id)))
        .exec(tx)
        .await?;
    Ok(())
}
async fn fail(state: &AppState, id: Uuid, credential: Uuid, code: &str) -> ApiResult<()> {
    let tx = state.postgres.begin().await?;
    crate::database::lock_credential(&tx, credential).await?;
    let inv = invocation(&tx, id, credential).await?;
    if matches!(inv.state.as_str(), "verifying" | "executing") {
        release(&tx, &inv, "failed", code).await?;
    }
    tx.commit().await?;
    Ok(())
}
fn required(inv: &Invocation) -> ApiResult<PaymentRequired> {
    Ok(PaymentRequired {
        x402_version: 2,
        resource: serde_json::from_value(inv.resource.clone())?,
        accepts: vec![serde_json::from_value(inv.requirements.clone())?],
    })
}
fn payment_response(inv: &Invocation) -> ApiResult<Response> {
    let payment = required(inv)?;
    let mut response = (StatusCode::PAYMENT_REQUIRED, Json(&payment)).into_response();
    response.headers_mut().insert(
        "payment-required",
        HeaderValue::from_str(&x402::encode(&payment)?)
            .map_err(|_| ApiError::bad("invalid_payment_header"))?,
    );
    Ok(response)
}
pub(super) fn receipt(inv: Invocation) -> ApiResult<Response> {
    let settled = inv.state == "settled";
    let settlement = inv
        .settlement
        .map(serde_json::from_value::<Settlement>)
        .transpose()?;
    let header = if settled && inv.payment_method.as_deref() == Some("x402") {
        settlement.as_ref().map(x402::encode).transpose()?
    } else {
        None
    };
    let body = Receipt {
        id: inv.id,
        capability_id: inv.capability_id,
        state: inv.state,
        price: inv.price.to_string(),
        result: if settled { inv.result } else { None },
        settlement: if settled { settlement } else { None },
        error_code: inv.error_code,
        execution_ms: inv.execution_ms,
        result_bytes: inv.result_bytes,
        payment_issue: inv.payment_issue,
    };
    let code = if settled {
        StatusCode::OK
    } else if matches!(
        body.state.as_str(),
        "failed" | "expired" | "execution_failed"
    ) {
        StatusCode::CONFLICT
    } else {
        StatusCode::ACCEPTED
    };
    let mut response = (code, Json(body)).into_response();
    if let Some(header) = header {
        response.headers_mut().insert(
            "payment-response",
            HeaderValue::from_str(&header).map_err(|_| ApiError::bad("invalid_payment_header"))?,
        );
    }
    Ok(response)
}

#[utoipa::path(post,path="/v1/invocations/{id}",params(("id"=Uuid,Path)),responses((status=200,body=Receipt),(status=402,body=PaymentRequired),(status=202,body=Receipt)))]
/// # Errors
/// Rejects invalid credentials, expired quotes, invalid payments, and failed execution or database operations.
pub async fn invoke(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let c = identity::authenticate_invoker(&state, &headers).await?;
    let tx = state.postgres.begin().await?;
    let c = locked_credential(&tx, c.id, identity::invocation_scope(&c)?).await?;
    let inv = invocation(&tx, id, c.id).await?;
    if inv.payment_method.as_deref() == Some("dodo") {
        let response = receipt(inv)?;
        tx.commit().await?;
        return Ok(response);
    }
    if inv.state != "quoted" {
        let response = receipt(inv)?;
        tx.commit().await?;
        return Ok(response);
    }
    if inv.expires_at <= now() {
        release(&tx, &inv, "expired", "quote_expired").await?;
        tx.commit().await?;
        return Err(ApiError::new(StatusCode::CONFLICT, "quote_expired"));
    }
    let Some(header) = headers.get("payment-signature") else {
        let response = payment_response(&inv)?;
        tx.commit().await?;
        return Ok(response);
    };
    if headers.get_all("payment-signature").iter().count() != 1 {
        return Err(ApiError::bad("ambiguous_payment_header"));
    }
    let payload = x402::decode(
        header
            .to_str()
            .map_err(|_| ApiError::bad("invalid_payment_header"))?,
    )?;
    let requirements: PaymentRequirements = serde_json::from_value(inv.requirements.clone())?;
    let resource = serde_json::from_value(inv.resource.clone())?;
    let nonce = x402::validate(
        &payload,
        &requirements,
        &resource,
        &c.wallet,
        inv.expires_at,
        now(),
    )?;
    let update = InvocationsEntity::update_many()
        .col_expr(InvocationsColumn::State, Expr::val("verifying"))
        .col_expr(InvocationsColumn::PaymentMethod, Expr::val("x402"))
        .col_expr(InvocationsColumn::PaymentNonce, Expr::val((nonce).clone()))
        .col_expr(
            InvocationsColumn::PaymentPayload,
            Expr::val((serde_json::to_value(&payload)?).clone()),
        )
        .col_expr(InvocationsColumn::UpdatedAt, Expr::val(now()))
        .filter(Expr::col(InvocationsColumn::Id).eq(Expr::val(id)))
        .exec(&tx)
        .await;
    if update.as_ref().err().is_some_and(|e| {
        matches!(
            e.sql_err(),
            Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
        )
    }) {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "payment_authorization_already_used",
        ));
    }
    update?;
    tx.commit().await?;
    if let Err(error) = x402::verify(&state, &payload, &requirements).await {
        fail(&state, id, c.id, error.code).await?;
        return Err(error);
    }
    execute_and_settle(&state, id, c.id, inv, payload, requirements).await
}

fn settlement_rejection(response: Option<&serde_json::Value>) -> Option<String> {
    response
        .filter(|value| value["success"] == false)
        .map(|value| {
            value["errorReason"]
                .as_str()
                .filter(|reason| !reason.is_empty())
                .unwrap_or("facilitator_rejected_settlement")
                .chars()
                .take(200)
                .collect()
        })
}
async fn execute_and_settle(
    state: &AppState,
    id: Uuid,
    credential_id: Uuid,
    inv: Invocation,
    payload: PaymentPayload,
    requirements: PaymentRequirements,
) -> ApiResult<Response> {
    let changed = InvocationsEntity::update_many()
        .col_expr(InvocationsColumn::State, Expr::val("executing"))
        .col_expr(InvocationsColumn::StartedAt, Expr::val(now()))
        .col_expr(InvocationsColumn::UpdatedAt, Expr::val(now()))
        .filter(
            Expr::col(InvocationsColumn::Id)
                .eq(Expr::val(id))
                .and(Expr::col(InvocationsColumn::State).eq(Expr::val("verifying"))),
        )
        .exec(&state.postgres)
        .await?
        .rows_affected;
    if changed != 1 {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "invocation_state_changed",
        ));
    }
    let execution_started = std::time::Instant::now();
    let input = inv.input.clone();
    let result = match tokio::time::timeout(
        std::time::Duration::from_secs(state.settings.runtime.worker.execution_timeout_seconds),
        cell::execute_capability(state, inv.capability_id, &input),
    )
    .await
    {
        Ok(Ok(result)) => result,
        Ok(Err(error)) => {
            fail(state, id, credential_id, error.code).await?;
            return Err(error);
        }
        Err(_) => {
            fail(state, id, credential_id, "execution_timeout").await?;
            return Err(ApiError::unavailable("execution_timeout"));
        }
    };
    let execution_ms = i64::try_from(execution_started.elapsed().as_millis()).unwrap_or(i64::MAX);
    let result_bytes = i64::try_from(serde_json::to_vec(&result)?.len()).unwrap_or(i64::MAX);
    InvocationsEntity::update_many()
        .col_expr(InvocationsColumn::ExecutionMs, Expr::val(execution_ms))
        .col_expr(InvocationsColumn::ResultBytes, Expr::val(result_bytes))
        .col_expr(InvocationsColumn::CompletedAt, Expr::val(now()))
        .filter(
            Expr::col(InvocationsColumn::Id)
                .eq(Expr::val(id))
                .and(Expr::col(InvocationsColumn::State).eq(Expr::val("executing"))),
        )
        .exec(&state.postgres)
        .await?;
    // Persist both output and the recovery scan boundary before any settlement request.
    let block = match x402::block_number(state).await {
        Ok(block) => block,
        Err(e) => {
            fail(state, id, credential_id, e.code).await?;
            return Err(e);
        }
    };
    let changed = InvocationsEntity::update_many()
        .col_expr(InvocationsColumn::State, Expr::val("settling"))
        .col_expr(InvocationsColumn::Result, Expr::val((result).clone()))
        .col_expr(
            InvocationsColumn::SettlementBlock,
            Expr::val((block).clone()),
        )
        .col_expr(InvocationsColumn::UpdatedAt, Expr::val(now()))
        .filter(
            Expr::col(InvocationsColumn::Id)
                .eq(Expr::val(id))
                .and(Expr::col(InvocationsColumn::State).eq(Expr::val("executing"))),
        )
        .exec(&state.postgres)
        .await?
        .rows_affected;
    if changed != 1 {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "invocation_state_changed",
        ));
    }
    let settled = x402::settle(state, &payload, &requirements).await;
    // A timeout or provider error does not prove that funds did not move.
    let response = settled.ok().map(serde_json::to_value).transpose()?;
    InvocationsEntity::update_many()
        .col_expr(InvocationsColumn::State, Expr::val("settlement_unknown"))
        .col_expr(InvocationsColumn::Settlement, Expr::val((response).clone()))
        .col_expr(
            InvocationsColumn::ErrorCode,
            Expr::val(settlement_rejection(response.as_ref())),
        )
        .col_expr(InvocationsColumn::UpdatedAt, Expr::val(now()))
        .filter(
            Expr::col(InvocationsColumn::Id)
                .eq(Expr::val(id))
                .and(Expr::col(InvocationsColumn::State).eq(Expr::val("settling"))),
        )
        .exec(&state.postgres)
        .await?;
    reconcile_inner(state, id, credential_id).await
}

async fn release_unexecuted(
    state: &AppState,
    id: Uuid,
    credential: Uuid,
    payload: &PaymentPayload,
    requirements: &PaymentRequirements,
) -> ApiResult<()> {
    match x402::expired_without_payment(state, payload, requirements).await {
        Ok(true) => {}
        Ok(false) => return Ok(()),
        Err(error) => {
            tracing::warn!(invocation_id=%id, error=error.code, "Settlement expiry proof unavailable");
            return Ok(());
        }
    }
    let tx = state.postgres.begin().await?;
    crate::database::lock_credential(&tx, credential).await?;
    let current = invocation(&tx, id, credential).await?;
    if current.payment_method.as_deref() == Some("x402")
        && matches!(current.state.as_str(), "settling" | "settlement_unknown")
    {
        release(&tx, &current, "failed", "settlement_not_executed").await?;
    }
    tx.commit().await?;
    Ok(())
}

/// # Errors
/// Returns errors for missing invocations, malformed recovery data, or database failures.
pub async fn reconcile_inner(
    state: &AppState,
    id: Uuid,
    credential_id: Uuid,
) -> ApiResult<Response> {
    let inv = InvocationsEntity::find()
        .filter(
            Expr::col(InvocationsColumn::Id)
                .eq(Expr::val(id))
                .and(Expr::col(InvocationsColumn::CredentialId).eq(Expr::val(credential_id))),
        )
        .into_model::<Invocation>()
        .one(&state.postgres)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "invocation_not_found"))?;
    if inv.payment_method.as_deref() == Some("dodo") {
        return dodo::reconcile(state, id, credential_id).await;
    }
    if !matches!(inv.state.as_str(), "settling" | "settlement_unknown") {
        return receipt(inv);
    }
    let payload: PaymentPayload = serde_json::from_value(
        inv.payment_payload
            .clone()
            .ok_or_else(|| ApiError::unavailable("missing_payment_recovery_data"))?,
    )?;
    let requirements: PaymentRequirements = serde_json::from_value(inv.requirements.clone())?;
    let confirmed = match x402::confirm(state, &inv, &payload, &requirements).await {
        Ok(v) => v,
        Err(error) => {
            tracing::warn!(invocation_id=%id,error=error.code,"Settlement reconciliation pending");
            None
        }
    };
    if confirmed.is_none() {
        release_unexecuted(state, id, credential_id, &payload, &requirements).await?;
    }
    if let Some(settlement) = confirmed {
        let tx = state.postgres.begin().await?;
        // Settlement is finalized even if the credential expired or was revoked meanwhile.
        crate::database::lock_credential(&tx, credential_id).await?;
        let inv = invocation(&tx, id, credential_id).await?;
        if inv.state != "settled" {
            InvocationsEntity::update_many()
                .col_expr(InvocationsColumn::State, Expr::val("settled"))
                .col_expr(
                    InvocationsColumn::Settlement,
                    Expr::val((serde_json::to_value(&settlement)?).clone()),
                )
                .col_expr(InvocationsColumn::UpdatedAt, Expr::val(now()))
                .filter(Expr::col(InvocationsColumn::Id).eq(Expr::val(id)))
                .exec(&tx)
                .await?;
            AgentCredentialsEntity::update_many()
                .col_expr(
                    AgentCredentialsColumn::Reserved,
                    Expr::col(AgentCredentialsColumn::Reserved).sub(Expr::val(inv.price)),
                )
                .col_expr(
                    AgentCredentialsColumn::Spent,
                    Expr::col(AgentCredentialsColumn::Spent).add(Expr::val(inv.price)),
                )
                .filter(Expr::col(AgentCredentialsColumn::Id).eq(Expr::val(credential_id)))
                .exec(&tx)
                .await?;
            tx.execute(
                &SqlQuery::insert()
                    .into_table(CreatorSalesEntity)
                    .columns([
                        CreatorSalesColumn::InvocationId,
                        CreatorSalesColumn::CreatorWallet,
                        CreatorSalesColumn::PayerWallet,
                        CreatorSalesColumn::Amount,
                        CreatorSalesColumn::Network,
                        CreatorSalesColumn::TransactionHash,
                        CreatorSalesColumn::CreatedAt,
                    ])
                    .values_panic([
                        Expr::val(id),
                        Expr::val((identity::wallet(&requirements.pay_to)?).clone()),
                        Expr::val((identity::wallet(&payload.payload.authorization.from)?).clone()),
                        Expr::val(inv.price),
                        Expr::val((state.settings.runtime.payment.network()).clone()),
                        Expr::val(settlement.transaction.clone()),
                        Expr::val(now()),
                    ])
                    .to_owned(),
            )
            .await?;
        }
        tx.commit().await?;
    }
    let current = InvocationsEntity::find()
        .filter(
            Expr::col(InvocationsColumn::Id)
                .eq(Expr::val(id))
                .and(Expr::col(InvocationsColumn::CredentialId).eq(Expr::val(credential_id))),
        )
        .into_model::<Invocation>()
        .one(&state.postgres)
        .await?
        .ok_or_else(|| sea_orm::DbErr::RecordNotFound("required row missing".into()))?;
    receipt(current)
}
#[utoipa::path(post,path="/v1/invocations/{id}/reconcile",params(("id"=Uuid,Path)),responses((status=200,body=Receipt),(status=202,body=Receipt)))]
/// # Errors
/// Rejects invalid credentials and propagates invocation recovery or database failures.
pub async fn reconcile(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let c = identity::authenticate(&state, &headers, "receipts:read").await?;
    reconcile_inner(&state, id, c.id).await
}
#[utoipa::path(get,path="/v1/invocations/{id}",params(("id"=Uuid,Path)),responses((status=200,body=Receipt),(status=202,body=Receipt)))]
/// # Errors
/// Rejects invalid credentials, missing invocations, and database failures.
pub async fn get_receipt(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let c = identity::authenticate(&state, &headers, "receipts:read").await?;
    let inv = InvocationsEntity::find()
        .filter(
            Expr::col(InvocationsColumn::Id)
                .eq(Expr::val(id))
                .and(Expr::col(InvocationsColumn::CredentialId).eq(Expr::val(c.id))),
        )
        .into_model::<Invocation>()
        .one(&state.postgres)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "invocation_not_found"))?;
    receipt(inv)
}

#[cfg(test)]
mod rejection_tests {
    #[test]
    fn explicit_rejection_is_visible_but_uncertainty_is_preserved() {
        use serde_json::json;
        assert_eq!(
            super::settlement_rejection(Some(
                &json!({"success":false,"errorReason":"invalid_signature"})
            ))
            .as_deref(),
            Some("invalid_signature")
        );
        assert_eq!(
            super::settlement_rejection(Some(&json!({"success":false}))).as_deref(),
            Some("facilitator_rejected_settlement")
        );
        assert_eq!(
            super::settlement_rejection(Some(&json!({"success":true}))),
            None
        );
        assert_eq!(super::settlement_rejection(None), None);
    }
}
