pub mod dodo;
pub mod worker;
use sea_orm::{ConnectionTrait, FromQueryResult, TransactionTrait};
pub mod x402;

use crate::{
    features::{
        cell,
        common::{
            error::{ApiError, ApiResult},
            model::{
                ASSET, Capability, Credential, Invocation, NETWORK, PaymentPayload,
                PaymentRequired, PaymentRequirements, QuoteRequest, QuoteResponse, Receipt,
                RepositoryInput, Resource, Settlement, now,
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
use serde_json::json;
use uuid::Uuid;

type Tx = DatabaseTransaction;
pub(super) async fn locked_credential(tx: &Tx, id: Uuid, scope: &str) -> ApiResult<Credential> {
    let c = Credential::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        "SELECT * FROM agent_credentials WHERE id=$1 FOR UPDATE",
        vec![(id).into()],
    ))
    .one(tx)
    .await?
    .ok_or_else(|| sea_orm::DbErr::RecordNotFound("required row missing".into()))?;
    identity::authorize(&c, scope)?;
    Ok(c)
}
pub(super) async fn invocation(tx: &Tx, id: Uuid, credential: Uuid) -> ApiResult<Invocation> {
    Invocation::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        "SELECT * FROM invocations WHERE id=$1 AND credential_id=$2 FOR UPDATE",
        vec![(id).into(), (credential).into()],
    ))
    .one(tx)
    .await?
    .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "invocation_not_found"))
}
async fn release(tx: &Tx, inv: &Invocation, state: &str, error: &str) -> ApiResult<()> {
    (*tx)
        .execute_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            "UPDATE invocations SET state=$1,error_code=$2,updated_at=$3 WHERE id=$4",
            vec![
                (state).into(),
                (error).into(),
                (now()).into(),
                (inv.id).into(),
            ],
        ))
        .await?;
    (*tx)
        .execute_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            "UPDATE agent_credentials SET reserved=reserved-$1 WHERE id=$2",
            vec![(inv.price).into(), (inv.credential_id).into()],
        ))
        .await?;
    Ok(())
}
async fn fail(state: &AppState, id: Uuid, credential: Uuid, code: &str) -> ApiResult<()> {
    let tx = state.postgres.begin().await?;
    tx.query_one_raw(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        "SELECT id FROM agent_credentials WHERE id=$1 FOR UPDATE",
        vec![(credential).into()],
    ))
    .await?
    .ok_or_else(|| sea_orm::DbErr::RecordNotFound("required row missing".into()))?;
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

#[utoipa::path(post,path="/v1/quotes",request_body=QuoteRequest,responses((status=201,body=QuoteResponse)))]
/// # Errors
/// Rejects invalid credentials or input, conflicting idempotency keys, insufficient budgets, and unavailable payment infrastructure.
pub async fn quote(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<QuoteRequest>,
) -> ApiResult<Response> {
    let c = identity::authenticate(&state, &headers, "repository:analyze").await?;
    if headers.get_all("idempotency-key").iter().count() != 1 {
        return Err(ApiError::bad("uuid_idempotency_key_required"));
    }
    let key = headers
        .get("idempotency-key")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<Uuid>().ok())
        .ok_or_else(|| ApiError::bad("uuid_idempotency_key_required"))?;
    cell::validate(&request.input)?;
    // Do remote I/O before acquiring a credential row lock or a pool transaction.
    x402::supported(&state).await?;
    let fingerprint = identity::hash(&serde_json::to_vec(&request)?);
    let tx = state.postgres.begin().await?;
    let c = locked_credential(&tx, c.id, "repository:analyze").await?;
    let existing = Invocation::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        "SELECT * FROM invocations WHERE credential_id=$1 AND idempotency_key=$2",
        vec![(c.id).into(), (key).into()],
    ))
    .one(&tx)
    .await?;
    if let Some(inv) = existing {
        let original: Vec<u8> = tx
            .query_one_raw(sea_orm::Statement::from_sql_and_values(
                sea_orm::DbBackend::Postgres,
                "SELECT request_hash FROM invocations WHERE id=$1",
                vec![(inv.id).into()],
            ))
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound("required row missing".into()))?
            .try_get_by_index(0)?;
        if original != fingerprint {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "idempotency_key_reused_with_different_input",
            ));
        }
        let result = QuoteResponse {
            id: inv.id,
            expires_at: inv.expires_at,
            payment: required(&inv)?,
        };
        tx.commit().await?;
        return Ok((StatusCode::OK, Json(result)).into_response());
    }
    // Only unused quotes expire automatically. Uncertain settlements keep their reservations.
    let released:i64=tx.query_one_raw(sea_orm::Statement::from_sql_and_values(sea_orm::DbBackend::Postgres, "WITH expired AS (UPDATE invocations SET state='expired',error_code='quote_expired',updated_at=$1 WHERE credential_id=$2 AND state='quoted' AND expires_at<=$1 RETURNING price) SELECT COALESCE(sum(price),0)::bigint FROM expired", vec![(now()).into(), (c.id).into()])).await?.ok_or_else(|| sea_orm::DbErr::RecordNotFound("required row missing".into()))?.try_get_by_index(0)?;
    if released > 0 {
        tx.execute_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            "UPDATE agent_credentials SET reserved=reserved-$1 WHERE id=$2",
            vec![(released).into(), (c.id).into()],
        ))
        .await?;
    }
    let cap = Capability::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        "SELECT * FROM capabilities WHERE id=$1 AND active",
        vec![(request.capability_id).into()],
    ))
    .one(&tx)
    .await?
    .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "capability_not_found"))?;
    let reserved=tx.execute_raw(sea_orm::Statement::from_sql_and_values(sea_orm::DbBackend::Postgres, "UPDATE agent_credentials SET reserved=reserved+$1 WHERE id=$2 AND $1<=per_call_limit AND spent+reserved+$1<=total_limit", vec![(cap.price).into(), (c.id).into()])).await?.rows_affected();
    if reserved != 1 {
        return Err(ApiError::forbidden());
    }
    let id = Uuid::new_v4();
    let at = now();
    let expires_at = at + 120;
    let requirements = PaymentRequirements {
        scheme: "exact".into(),
        network: NETWORK.into(),
        amount: cap.price.to_string(),
        asset: ASSET.into(),
        pay_to: cap.creator_wallet,
        max_timeout_seconds: 120,
        extra: json!({"name":"USDC","version":"2"}),
    };
    let resource = Resource {
        url: format!("{}/v1/invocations/{id}", state.settings.public_url),
        description: cap.description,
        mime_type: "application/json".into(),
    };
    tx.execute_raw(sea_orm::Statement::from_sql_and_values(sea_orm::DbBackend::Postgres, "INSERT INTO invocations(id,credential_id,capability_id,idempotency_key,request_hash,input,requirements,resource,state,price,expires_at,created_at,updated_at,creator_wallet) VALUES($1,$2,$3,$4,$5,$6,$7,$8,'quoted',$9,$10,$11,$11,$12)", vec![(id).into(), (c.id).into(), (cap.id).into(), (key).into(), (fingerprint).into(), (serde_json::to_value(request.input)?).into(), (serde_json::to_value(&requirements)?).into(), (serde_json::to_value(&resource)?).into(), (cap.price).into(), (expires_at).into(), (at).into(), (requirements.pay_to.clone()).into()])).await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(QuoteResponse {
            id,
            expires_at,
            payment: PaymentRequired {
                x402_version: 2,
                resource,
                accepts: vec![requirements],
            },
        }),
    )
        .into_response())
}

#[utoipa::path(post,path="/v1/invocations/{id}",params(("id"=Uuid,Path)),responses((status=200,body=Receipt),(status=402,body=PaymentRequired),(status=202,body=Receipt)))]
/// # Errors
/// Rejects invalid credentials, expired quotes, invalid payments, and failed execution or database operations.
pub async fn invoke(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let c = identity::authenticate(&state, &headers, "repository:analyze").await?;
    let tx = state.postgres.begin().await?;
    let c = locked_credential(&tx, c.id, "repository:analyze").await?;
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
    let update=tx.execute_raw(sea_orm::Statement::from_sql_and_values(sea_orm::DbBackend::Postgres, "UPDATE invocations SET state='verifying',payment_method='x402',payment_nonce=$1,payment_payload=$2,updated_at=$3 WHERE id=$4", vec![(nonce).into(), (serde_json::to_value(&payload)?).into(), (now()).into(), (id).into()])).await;
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

async fn execute_and_settle(
    state: &AppState,
    id: Uuid,
    credential_id: Uuid,
    inv: Invocation,
    payload: PaymentPayload,
    requirements: PaymentRequirements,
) -> ApiResult<Response> {
    let changed=state.postgres.execute_raw(sea_orm::Statement::from_sql_and_values(sea_orm::DbBackend::Postgres, "UPDATE invocations SET state='executing',started_at=$1,updated_at=$1 WHERE id=$2 AND state='verifying'", vec![(now()).into(), (id).into()])).await?.rows_affected();
    if changed != 1 {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "invocation_state_changed",
        ));
    }
    let execution_started = std::time::Instant::now();
    let input: RepositoryInput = serde_json::from_value(inv.input.clone())?;
    let result = match tokio::time::timeout(
        std::time::Duration::from_secs(60),
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
    state.postgres.execute_raw(sea_orm::Statement::from_sql_and_values(sea_orm::DbBackend::Postgres,"UPDATE invocations SET execution_ms=$1,result_bytes=$2,completed_at=$3 WHERE id=$4 AND state='executing'",[execution_ms.into(),result_bytes.into(),now().into(),id.into()])).await?;
    // Persist both output and the recovery scan boundary before any settlement request.
    let block = match x402::block_number(state).await {
        Ok(block) => block,
        Err(e) => {
            fail(state, id, credential_id, e.code).await?;
            return Err(e);
        }
    };
    let changed=state.postgres.execute_raw(sea_orm::Statement::from_sql_and_values(sea_orm::DbBackend::Postgres, "UPDATE invocations SET state='settling',result=$1,settlement_block=$2,updated_at=$3 WHERE id=$4 AND state='executing'", vec![(result).into(), (block).into(), (now()).into(), (id).into()])).await?.rows_affected();
    if changed != 1 {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "invocation_state_changed",
        ));
    }
    let settled = x402::settle(state, &payload, &requirements).await;
    // A timeout or provider error does not prove that funds did not move.
    let response = settled.ok().map(serde_json::to_value).transpose()?;
    state.postgres.execute_raw(sea_orm::Statement::from_sql_and_values(sea_orm::DbBackend::Postgres, "UPDATE invocations SET state='settlement_unknown',settlement=$1,updated_at=$2 WHERE id=$3 AND state='settling'", vec![(response).into(), (now()).into(), (id).into()])).await?;
    reconcile_inner(state, id, credential_id).await
}

/// # Errors
/// Returns errors for missing invocations, malformed recovery data, or database failures.
pub async fn reconcile_inner(
    state: &AppState,
    id: Uuid,
    credential_id: Uuid,
) -> ApiResult<Response> {
    let inv = Invocation::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        "SELECT * FROM invocations WHERE id=$1 AND credential_id=$2",
        vec![(id).into(), (credential_id).into()],
    ))
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
    if let Some(settlement) = confirmed {
        let tx = state.postgres.begin().await?;
        // Settlement is finalized even if the credential expired or was revoked meanwhile.
        tx.query_one_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            "SELECT id FROM agent_credentials WHERE id=$1 FOR UPDATE",
            vec![(credential_id).into()],
        ))
        .await?
        .ok_or_else(|| sea_orm::DbErr::RecordNotFound("required row missing".into()))?;
        let inv = invocation(&tx, id, credential_id).await?;
        if inv.state != "settled" {
            tx.execute_raw(sea_orm::Statement::from_sql_and_values(
                sea_orm::DbBackend::Postgres,
                "UPDATE invocations SET state='settled',settlement=$1,updated_at=$2 WHERE id=$3",
                vec![
                    (serde_json::to_value(&settlement)?).into(),
                    (now()).into(),
                    (id).into(),
                ],
            ))
            .await?;
            tx.execute_raw(sea_orm::Statement::from_sql_and_values(
                sea_orm::DbBackend::Postgres,
                "UPDATE agent_credentials SET reserved=reserved-$1,spent=spent+$1 WHERE id=$2",
                vec![(inv.price).into(), (credential_id).into()],
            ))
            .await?;
            tx.execute_raw(sea_orm::Statement::from_sql_and_values(sea_orm::DbBackend::Postgres, "INSERT INTO creator_sales(invocation_id,creator_wallet,payer_wallet,amount,network,transaction_hash,created_at) VALUES($1,$2,$3,$4,$5,$6,$7)", vec![(id).into(), (identity::wallet(&requirements.pay_to)?).into(), (identity::wallet(&payload.payload.authorization.from)?).into(), (inv.price).into(), (NETWORK).into(), (&settlement.transaction).into(), (now()).into()])).await?;
        }
        tx.commit().await?;
    }
    let current = Invocation::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        "SELECT * FROM invocations WHERE id=$1 AND credential_id=$2",
        vec![(id).into(), (credential_id).into()],
    ))
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
    let inv = Invocation::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        "SELECT * FROM invocations WHERE id=$1 AND credential_id=$2",
        vec![(id).into(), (c.id).into()],
    ))
    .one(&state.postgres)
    .await?
    .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "invocation_not_found"))?;
    receipt(inv)
}
