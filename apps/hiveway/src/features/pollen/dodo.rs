//! Dodo test checkout and authenticated webhook inbox. Canonical provider reads
//! establish payment status; neither redirects nor client-supplied receipts do.
use super::{invocation, locked_credential, receipt, x402};
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
use sea_orm::{ConnectionTrait, DbBackend, FromQueryResult, Statement, TransactionTrait};
use serde_json::{Value, json};
use uuid::Uuid;

fn statement(sql: &str, values: Vec<sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}
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
    tx.execute_raw(statement("UPDATE invocations SET payment_method='dodo',state='checkout_creating',updated_at=$1 WHERE id=$2",vec![now().into(),id.into()])).await?;
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
    state.postgres.execute_raw(statement("UPDATE invocations SET state='checkout_pending',checkout_id=$1,checkout_url=$2,updated_at=$3 WHERE id=$4 AND state='checkout_creating'",vec![session.into(),url.into(),now().into(),id.into()])).await?;
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
    state.postgres.execute_raw(statement("INSERT INTO payment_webhook_inbox(event_id,event_type,payment_id,received_at) VALUES($1,$2,$3,$4) ON CONFLICT(event_id) DO NOTHING",vec![id.into(),kind.into(),payment.into(),now().into()])).await?;
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
    let inv = Invocation::find_by_statement(statement(
        "SELECT * FROM invocations WHERE id=$1 AND credential_id=$2",
        vec![id.into(), credential.into()],
    ))
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
    let credential_wallet = state
        .postgres
        .query_one_raw(statement(
            "SELECT wallet FROM agent_credentials WHERE id=$1",
            vec![credential.into()],
        ))
        .await?
        .ok_or_else(|| ApiError::bad("missing_credential"))?
        .try_get::<String>("", "wallet")?;
    if payment["metadata"]["hive_wallet"] != credential_wallet {
        return Err(ApiError::bad("dodo_payment_binding_mismatch"));
    }
    let tx = state.postgres.begin().await?;
    tx.execute_raw(statement(
        "SELECT id FROM agent_credentials WHERE id=$1 FOR UPDATE",
        vec![credential.into()],
    ))
    .await?;
    let current = invocation(&tx, id, credential).await?;
    if current.state == "checkout_pending" {
        // The merchant receives USD. This ledger entry is not an on-chain transfer
        // to the creator and creator payouts require merchant payout operations.
        tx.execute_raw(statement(
            "UPDATE agent_credentials SET reserved=reserved-$1,spent=spent+$1 WHERE id=$2",
            vec![inv.price.into(), credential.into()],
        ))
        .await?;
        tx.execute_raw(statement(
            "UPDATE invocations SET state='paid',provider_payment_id=$1,updated_at=$2 WHERE id=$3",
            vec![payment_id.into(), now().into(), id.into()],
        ))
        .await?;
    }
    tx.commit().await?;
    let current = Invocation::find_by_statement(statement(
        "SELECT * FROM invocations WHERE id=$1",
        vec![id.into()],
    ))
    .one(&state.postgres)
    .await?
    .ok_or_else(|| ApiError::bad("invocation_not_found"))?;
    receipt(current)
}
/// Process one stored event using canonical provider reads.
/// # Errors
/// Returns database or provider failures; events remain retryable.
pub async fn process_events(state: &AppState) -> ApiResult<()> {
    let events=state.postgres.query_all_raw(statement("SELECT event_id,payment_id,event_type FROM payment_webhook_inbox WHERE processed_at IS NULL AND attempts<20 ORDER BY received_at LIMIT 20",vec![])).await?;
    for event in events {
        let id: String = event.try_get("", "event_id")?;
        let payment: Option<String> = event.try_get("", "payment_id")?;
        let kind: String = event.try_get("", "event_type")?;
        let result=async {
            if let Some(payment_id)=payment {
                identifier(&payment_id)?;
                let payment=request(state,reqwest::Method::GET,&format!("/payments/{payment_id}"),None).await?;
                if let Some(invocation_id)=payment["metadata"]["hive_invocation_id"].as_str().and_then(|v|v.parse::<Uuid>().ok()){
                    let inv=Invocation::find_by_statement(statement("SELECT * FROM invocations WHERE id=$1 AND payment_method='dodo'",vec![invocation_id.into()])).one(&state.postgres).await?;
                    if let Some(inv)=inv{
                        if kind=="payment.succeeded"{reconcile(state,inv.id,inv.credential_id).await?;}
                        else if kind.contains("refund")||kind.contains("dispute"){
                            state.postgres.execute_raw(statement("UPDATE invocations SET payment_issue=$1,updated_at=$2 WHERE id=$3",vec![kind.clone().into(),now().into(),inv.id.into()])).await?;
                        }
                    }
                }
            }
            Ok::<(),ApiError>(())
        }.await;
        state.postgres.execute_raw(statement("UPDATE payment_webhook_inbox SET attempts=attempts+1,processed_at=$1,last_error=$2 WHERE event_id=$3",vec![result.as_ref().ok().map(|()|now()).into(),result.err().map(|e|e.code.to_owned()).into(),id.into()])).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::identifier;
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
