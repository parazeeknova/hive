//! Dodo test checkout and authenticated webhook inbox. Canonical provider reads
//! establish payment status; neither redirects nor client-supplied receipts do.
use super::{invocation, locked_credential, receipt, x402};
use crate::entities::agent_credentials::Column as AgentCredentialsColumn;
use crate::entities::agent_credentials::Entity as AgentCredentialsEntity;
use crate::entities::invocations::Column as InvocationsColumn;
use crate::entities::invocations::Entity as InvocationsEntity;
use crate::entities::payment_webhook_inbox::Column as PaymentWebhookInboxColumn;
use crate::entities::payment_webhook_inbox::Entity as PaymentWebhookInboxEntity;
use crate::{
    features::{
        common::{
            error::{ApiError, ApiResult},
            model::{Invocation, now},
        },
        identity,
    },
    state::AppState,
};
use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Response,
};
use sea_orm::sea_query::{Expr, ExprTrait, OnConflict, Query as SqlQuery};
use sea_orm::{ColumnTrait, QueryOrder, QuerySelect};
use sea_orm::{ConnectionTrait, EntityTrait, QueryFilter, TransactionTrait};
use serde_json::{Value, json};
use uuid::Uuid;

async fn request(
    state: &AppState,
    method: reqwest::Method,
    path: &str,
    body: Option<Value>,
) -> ApiResult<Value> {
    let key = state
        .settings
        .dodo_api_key
        .as_ref()
        .ok_or_else(|| ApiError::unavailable("dodo_not_configured"))?;
    let mut request = state
        .http
        .request(method, format!("{}{path}", state.settings.dodo_api_url))
        .bearer_auth(key);
    if let Some(body) = body {
        request = request.json(&body);
    }
    x402::json_response(
        request
            .send()
            .await
            .map_err(|_| ApiError::unavailable("dodo_unavailable"))?,
    )
    .await
}
fn identifier(value: &str) -> ApiResult<&str> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err(ApiError::bad("invalid_provider_id"));
    }
    Ok(value)
}
#[utoipa::path(post,path="/v1/invocations/{id}/checkout",params(("id"=Uuid,Path)),responses((status=200,description="Hosted test checkout")))]
/// # Errors
/// Rejects invalid credentials, incompatible prices, unavailable configuration, and provider failures.
pub async fn checkout(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    let c = identity::authenticate(&state, &headers, "repository:analyze").await?;
    let product = state
        .settings
        .dodo_product_id
        .as_ref()
        .ok_or_else(|| ApiError::unavailable("dodo_product_not_configured"))?;
    identifier(product)?;
    if state.settings.dodo_api_key.is_none() {
        return Err(ApiError::unavailable("dodo_not_configured"));
    }
    let tx = state.postgres.begin().await?;
    locked_credential(&tx, c.id, "repository:analyze").await?;
    let inv = invocation(&tx, id, c.id).await?;
    if inv.payment_method.as_deref() == Some("dodo") {
        let response = json!({"invocation_id":id,"state":inv.state,"checkout_id":inv.checkout_id,"checkout_url":inv.checkout_url});
        tx.commit().await?;
        return Ok(Json(response));
    }
    if inv.state != "quoted" || inv.expires_at <= now() {
        return Err(ApiError::bad("quote_not_payable"));
    }
    if inv.price % 10_000 != 0 || inv.price < 10_000 {
        return Err(ApiError::bad("dodo_requires_whole_usd_cents"));
    }
    // Claim before the provider call. Uncertain checkout creation is never retried
    // automatically, since a lost response could otherwise create duplicate carts.
    InvocationsEntity::update_many()
        .col_expr(InvocationsColumn::PaymentMethod, Expr::val("dodo"))
        .col_expr(InvocationsColumn::State, Expr::val("checkout_creating"))
        .col_expr(InvocationsColumn::UpdatedAt, Expr::val(now()))
        .filter(Expr::col(InvocationsColumn::Id).eq(Expr::val(id)))
        .exec(&tx)
        .await?;
    tx.commit().await?;
    let response=request(&state,reqwest::Method::POST,"/checkouts",Some(json!({"product_cart":[{"product_id":product,"quantity":1,"amount":inv.price/10_000}],"metadata":{"hive_invocation_id":id.to_string(),"hive_wallet":c.wallet},"feature_flags":{"allow_currency_selection":false,"allow_discount_code":false}}))).await?;
    let session = response["session_id"]
        .as_str()
        .ok_or_else(|| ApiError::unavailable("invalid_dodo_checkout"))?;
    identifier(session)?;
    let url = response["checkout_url"]
        .as_str()
        .ok_or_else(|| ApiError::unavailable("missing_dodo_checkout_url"))?;
    let parsed =
        reqwest::Url::parse(url).map_err(|_| ApiError::unavailable("invalid_dodo_checkout_url"))?;
    if parsed.scheme() != "https"
        || parsed
            .host_str()
            .is_none_or(|host| !host.ends_with(".dodopayments.com"))
    {
        return Err(ApiError::unavailable("invalid_dodo_checkout_url"));
    }
    InvocationsEntity::update_many()
        .col_expr(InvocationsColumn::State, Expr::val("checkout_pending"))
        .col_expr(InvocationsColumn::CheckoutId, Expr::val(session))
        .col_expr(InvocationsColumn::CheckoutUrl, Expr::val(url))
        .col_expr(InvocationsColumn::UpdatedAt, Expr::val(now()))
        .filter(
            Expr::col(InvocationsColumn::Id)
                .eq(Expr::val(id))
                .and(Expr::col(InvocationsColumn::State).eq(Expr::val("checkout_creating"))),
        )
        .exec(&state.postgres)
        .await?;
    Ok(Json(
        json!({"invocation_id":id,"state":"checkout_pending","checkout_id":session,"checkout_url":url}),
    ))
}
#[utoipa::path(post,path="/v1/webhooks/dodo",request_body(content=String,content_type="application/json"),responses((status=200,description="Verified event stored")))]
/// # Errors
/// Rejects invalid signatures, timestamps, event envelopes, and database failures.
pub async fn webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<StatusCode> {
    for name in ["webhook-id", "webhook-timestamp", "webhook-signature"] {
        if headers.get_all(name).iter().count() != 1 {
            return Err(ApiError::new(
                StatusCode::UNAUTHORIZED,
                "ambiguous_webhook_headers",
            ));
        }
    }
    let secret = state
        .settings
        .dodo_webhook_secret
        .as_ref()
        .ok_or_else(|| ApiError::unavailable("dodo_webhook_not_configured"))?;
    standardwebhooks::Webhook::new(secret)
        .map_err(|_| ApiError::unavailable("invalid_webhook_configuration"))?
        .verify(&body, &headers)
        .map_err(|_| ApiError::new(StatusCode::UNAUTHORIZED, "invalid_webhook_signature"))?;
    let event: Value =
        serde_json::from_slice(&body).map_err(|_| ApiError::bad("invalid_webhook_body"))?;
    let id = headers
        .get("webhook-id")
        .and_then(|v| v.to_str().ok())
        .filter(|s| s.len() <= 256)
        .ok_or_else(|| ApiError::bad("invalid_webhook_id"))?;
    let kind = event["type"]
        .as_str()
        .filter(|v| v.len() <= 100)
        .ok_or_else(|| ApiError::bad("invalid_webhook_event"))?;
    let payment = event["data"]["payment_id"]
        .as_str()
        .map(identifier)
        .transpose()?;
    state
        .postgres
        .execute(
            &SqlQuery::insert()
                .into_table(PaymentWebhookInboxEntity)
                .columns([
                    PaymentWebhookInboxColumn::EventId,
                    PaymentWebhookInboxColumn::EventType,
                    PaymentWebhookInboxColumn::PaymentId,
                    PaymentWebhookInboxColumn::ReceivedAt,
                ])
                .values_panic([
                    Expr::val(id),
                    Expr::val(kind),
                    Expr::val(payment),
                    Expr::val(now()),
                ])
                .on_conflict(
                    OnConflict::columns([PaymentWebhookInboxColumn::EventId])
                        .do_nothing()
                        .to_owned(),
                )
                .to_owned(),
        )
        .await?;
    Ok(StatusCode::OK)
}
fn matches_payment(payment: &Value, session: &Value, inv: &Invocation, product: &str) -> bool {
    payment["status"] == "succeeded"
        && payment["currency"] == "USD"
        && payment["total_amount"]
            .as_i64()
            .is_some_and(|n| n == inv.price / 10_000)
        && payment["metadata"]["hive_invocation_id"] == inv.id.to_string()
        && payment["metadata"]["hive_wallet"]
            .as_str()
            .is_some_and(|w| identity::wallet(w).is_ok())
        && payment["checkout_session_id"] == inv.checkout_id.as_deref().unwrap_or_default()
        && session["id"] == inv.checkout_id.as_deref().unwrap_or_default()
        && session["payment_id"] == payment["payment_id"]
        && payment["product_cart"].as_array().is_some_and(|cart| {
            cart.len() == 1 && cart[0]["product_id"] == product && cart[0]["quantity"] == 1
        })
}
/// Re-read a checkout and its canonical payment; callbacks cannot mark invoices paid.
/// # Errors
/// Returns provider, ownership, payment binding, or database errors.
pub async fn reconcile(state: &AppState, id: Uuid, credential: Uuid) -> ApiResult<Response> {
    let inv = InvocationsEntity::find()
        .filter(
            Expr::col(InvocationsColumn::Id)
                .eq(Expr::val(id))
                .and(Expr::col(InvocationsColumn::CredentialId).eq(Expr::val(credential))),
        )
        .into_model::<Invocation>()
        .one(&state.postgres)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "invocation_not_found"))?;
    if inv.payment_method.as_deref() != Some("dodo") || inv.state != "checkout_pending" {
        return receipt(inv);
    }
    let session_id = inv
        .checkout_id
        .as_deref()
        .ok_or_else(|| ApiError::bad("missing_checkout_id"))?;
    identifier(session_id)?;
    let session = request(
        state,
        reqwest::Method::GET,
        &format!("/checkouts/{session_id}"),
        None,
    )
    .await?;
    let Some(payment_id) = session["payment_id"].as_str() else {
        return receipt(inv);
    };
    identifier(payment_id)?;
    let payment = request(
        state,
        reqwest::Method::GET,
        &format!("/payments/{payment_id}"),
        None,
    )
    .await?;
    let product = state
        .settings
        .dodo_product_id
        .as_deref()
        .ok_or_else(|| ApiError::unavailable("dodo_product_not_configured"))?;
    if payment["status"] != "succeeded" {
        return receipt(inv);
    }
    if !matches_payment(&payment, &session, &inv, product) {
        return Err(ApiError::bad("dodo_payment_binding_mismatch"));
    }
    let tx = state.postgres.begin().await?;
    let owner = crate::database::lock_credential(&tx, credential).await?;
    if payment["metadata"]["hive_wallet"] != owner.wallet {
        return Err(ApiError::bad("dodo_payment_binding_mismatch"));
    }
    let current = invocation(&tx, id, credential).await?;
    if current.state == "checkout_pending" {
        // The merchant receives USD. This ledger entry is not an on-chain transfer
        // to the creator and creator payouts require merchant payout operations.
        AgentCredentialsEntity::update_many()
            .col_expr(
                AgentCredentialsColumn::Reserved,
                Expr::col(AgentCredentialsColumn::Reserved).sub(Expr::val(inv.price)),
            )
            .col_expr(
                AgentCredentialsColumn::Spent,
                Expr::col(AgentCredentialsColumn::Spent).add(Expr::val(inv.price)),
            )
            .filter(Expr::col(AgentCredentialsColumn::Id).eq(Expr::val(credential)))
            .exec(&tx)
            .await?;
        InvocationsEntity::update_many()
            .col_expr(InvocationsColumn::State, Expr::val("paid"))
            .col_expr(InvocationsColumn::ProviderPaymentId, Expr::val(payment_id))
            .col_expr(InvocationsColumn::UpdatedAt, Expr::val(now()))
            .filter(Expr::col(InvocationsColumn::Id).eq(Expr::val(id)))
            .exec(&tx)
            .await?;
    }
    tx.commit().await?;
    let current = InvocationsEntity::find()
        .filter(Expr::col(InvocationsColumn::Id).eq(Expr::val(id)))
        .into_model::<Invocation>()
        .one(&state.postgres)
        .await?
        .ok_or_else(|| ApiError::bad("invocation_not_found"))?;
    receipt(current)
}
/// Process one stored event using canonical provider reads.
/// # Errors
/// Returns database or provider failures; events remain retryable.
pub async fn process_events(state: &AppState) -> ApiResult<()> {
    let events = PaymentWebhookInboxEntity::find()
        .filter(PaymentWebhookInboxColumn::ProcessedAt.is_null())
        .filter(
            PaymentWebhookInboxColumn::Attempts.lt(state
                .settings
                .runtime
                .worker
                .webhook_max_attempts),
        )
        .order_by_asc(PaymentWebhookInboxColumn::ReceivedAt)
        .limit(crate::database::limit(
            state.settings.runtime.worker.batch_size,
        )?)
        .all(&state.postgres)
        .await?;
    for event in events {
        let id = event.event_id;
        let payment = event.payment_id;
        let kind = event.event_type;
        let result = async {
            if let Some(payment_id) = payment {
                identifier(&payment_id)?;
                let payment = request(
                    state,
                    reqwest::Method::GET,
                    &format!("/payments/{payment_id}"),
                    None,
                )
                .await?;
                if let Some(invocation_id) = payment["metadata"]["hive_invocation_id"]
                    .as_str()
                    .and_then(|v| v.parse::<Uuid>().ok())
                {
                    let inv = InvocationsEntity::find()
                        .filter(
                            Expr::col(InvocationsColumn::Id)
                                .eq(Expr::val(invocation_id))
                                .and(
                                    Expr::col(InvocationsColumn::PaymentMethod)
                                        .eq(Expr::val("dodo")),
                                ),
                        )
                        .into_model::<Invocation>()
                        .one(&state.postgres)
                        .await?;
                    if let Some(inv) = inv {
                        if kind == "payment.succeeded" {
                            reconcile(state, inv.id, inv.credential_id).await?;
                        } else if kind.contains("refund") || kind.contains("dispute") {
                            InvocationsEntity::update_many()
                                .col_expr(
                                    InvocationsColumn::PaymentIssue,
                                    Expr::val((kind.clone()).clone()),
                                )
                                .col_expr(InvocationsColumn::UpdatedAt, Expr::val(now()))
                                .filter(Expr::col(InvocationsColumn::Id).eq(Expr::val(inv.id)))
                                .exec(&state.postgres)
                                .await?;
                        }
                    }
                }
            }
            Ok::<(), ApiError>(())
        }
        .await;
        PaymentWebhookInboxEntity::update_many()
            .col_expr(
                PaymentWebhookInboxColumn::Attempts,
                Expr::col(PaymentWebhookInboxColumn::Attempts).add(Expr::val(1_i64)),
            )
            .col_expr(
                PaymentWebhookInboxColumn::ProcessedAt,
                Expr::val(result.as_ref().ok().map(|()| now())),
            )
            .col_expr(
                PaymentWebhookInboxColumn::LastError,
                Expr::val((result.err().map(|e| e.code.to_owned())).clone()),
            )
            .filter(Expr::col(PaymentWebhookInboxColumn::EventId).eq(Expr::val((id).clone())))
            .exec(&state.postgres)
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::identifier;
    #[cfg(not(miri))]
    use axum::http::{HeaderMap, HeaderValue};
    #[test]
    fn canonical_checkout_rejects_amount_and_session_tampering()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::features::common::model::Invocation;
        use serde_json::json;
        use uuid::Uuid;
        let id = Uuid::new_v4();
        let inv = Invocation {
            id,
            credential_id: Uuid::new_v4(),
            capability_id: Uuid::new_v4(),
            input: json!({}),
            requirements: json!({}),
            resource: json!({}),
            state: "checkout_pending".into(),
            price: 1_000_000,
            expires_at: 0,
            payment_nonce: None,
            payment_payload: None,
            result: None,
            settlement: None,
            settlement_block: None,
            error_code: None,
            payment_method: Some("dodo".into()),
            checkout_id: Some("cks_1".into()),
            checkout_url: None,
            provider_payment_id: None,
            worker_started_at: None,
            execution_ms: None,
            result_bytes: None,
            payment_issue: None,
        };
        let session = json!({"id":"cks_1","payment_id":"pay_1"});
        let original = json!({"status":"succeeded","currency":"USD","total_amount":100,"payment_id":"pay_1","checkout_session_id":"cks_1","metadata":{"hive_invocation_id":id.to_string(),"hive_wallet":"0x1111111111111111111111111111111111111111"},"product_cart":[{"product_id":"prod_1","quantity":1}]});
        assert!(super::matches_payment(&original, &session, &inv, "prod_1"));
        for (path, value) in [
            ("/total_amount", json!(99)),
            ("/total_amount", json!(200)),
            ("/currency", json!("EUR")),
            ("/checkout_session_id", json!("cks_other")),
            ("/payment_id", json!("pay_other")),
            ("/product_cart/0/product_id", json!("prod_other")),
            ("/product_cart/0/quantity", json!(2)),
        ] {
            let mut altered = original.clone();
            *altered.pointer_mut(path).ok_or("missing fixture field")? = value;
            assert!(
                !super::matches_payment(&altered, &session, &inv, "prod_1"),
                "accepted altered payment: {path}"
            );
        }
        Ok(())
    }
    #[test]
    fn provider_ids_cannot_escape_api_paths() {
        for value in ["../payments/other", "id?secret=x", "id/another", ""] {
            assert!(identifier(value).is_err());
        }
        assert!(identifier("pay_abc123").is_ok());
    }
    // The upstream verifier requires a real wall clock; native HTTP tests cover it.
    #[cfg(not(miri))]
    #[test]
    fn signatures_bind_raw_payload_and_timestamp() -> Result<(), Box<dyn std::error::Error>> {
        use base64::Engine;
        let secret = base64::engine::general_purpose::STANDARD.encode([3; 32]);
        let verifier = standardwebhooks::Webhook::new(&secret)?;
        let timestamp = time::OffsetDateTime::now_utc().unix_timestamp();
        let payload = br#"{"type":"payment.succeeded","data":{"payment_id":"pay_1"}}"#;
        let signed = verifier.sign("msg_1", timestamp, payload)?;
        let mut headers = HeaderMap::new();
        headers.insert("webhook-id", HeaderValue::from_static("msg_1"));
        headers.insert(
            "webhook-timestamp",
            HeaderValue::from_str(&timestamp.to_string())?,
        );
        headers.insert("webhook-signature", HeaderValue::from_str(&signed)?);
        verifier.verify(payload, &headers)?;
        assert!(verifier.verify(b"changed payment", &headers).is_err());
        headers.insert("webhook-timestamp", HeaderValue::from_static("1"));
        assert!(verifier.verify(payload, &headers).is_err());
        Ok(())
    }
}
