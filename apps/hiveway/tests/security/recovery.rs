use super::*;
use hiveway::{entities::invocations, features::pollen};
use sea_orm::sea_query::{Expr, ExprTrait};
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Default)]
pub(super) struct ChainFixture(pub AtomicU8);
pub(super) async fn rpc(
    axum::extract::State(fixture): axum::extract::State<Arc<ChainFixture>>,
    axum::Json(request): axum::Json<Value>,
) -> axum::Json<Value> {
    let mode = fixture.0.load(Ordering::Relaxed);
    let result = match request["method"].as_str().unwrap_or_default() {
        "eth_getLogs" => json!([]),
        "eth_chainId" => json!(if mode == 5 { "0x1" } else { "0x14a34" }),
        "eth_getBlockByNumber" => {
            assert_eq!(request["params"], json!(["finalized", false]));
            if mode == 6 {
                Value::Null
            } else {
                let timestamp = match mode {
                    0 => 250,
                    7 => 260,
                    _ => 1000,
                };
                json!({"hash":format!("0x{}", "11".repeat(32)),"timestamp":format!("0x{timestamp:x}")})
            }
        }
        "eth_call" => {
            assert_eq!(
                request["params"][1],
                json!({"blockHash":format!("0x{}", "11".repeat(32)),"requireCanonical":true})
            );
            assert_eq!(
                request["params"][0]["data"].as_str().map(str::len),
                Some(138)
            );
            if mode == 4 {
                return axum::Json(
                    json!({"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"canonical state unavailable"}}),
                );
            }
            if mode == 3 {
                json!("0x")
            } else {
                json!(format!("0x{:0>64}", if mode == 1 { "1" } else { "0" }))
            }
        }
        other => panic!("unexpected fixture RPC method {other}"),
    };
    axum::Json(json!({"jsonrpc":"2.0","id":1,"result":result}))
}

pub(super) async fn expiry(
    state: &AppState,
    fixture: &ChainFixture,
    credential: Uuid,
    capability: Uuid,
) -> Result {
    for (mode, original) in [
        (0, "settling"),
        (1, "settling"),
        (3, "settling"),
        (4, "settling"),
        (5, "settling"),
        (6, "settling"),
        (7, "settling"),
        (2, "settling"),
        (2, "settlement_unknown"),
    ] {
        fixture.0.store(mode, Ordering::Relaxed);
        let before = agent_credentials::Entity::find_by_id(credential)
            .one(&state.postgres)
            .await?
            .ok_or("missing credential")?
            .reserved;
        let id = pending(state, credential, capability, original).await?;
        if mode == 2 {
            let inv = invocations::Entity::find_by_id(id)
                .one(&state.postgres)
                .await?
                .ok_or("missing invocation")?;
            let payload = serde_json::from_value(inv.payment_payload.ok_or("missing payload")?)?;
            let requirements = serde_json::from_value(inv.requirements)?;
            assert!(pollen::x402::expired_without_payment(state, &payload, &requirements).await?);
        }
        let (first, retry) = tokio::join!(
            pollen::reconcile_inner(state, id, credential),
            pollen::reconcile_inner(state, id, credential)
        );
        first?;
        retry?;
        let inv = invocations::Entity::find_by_id(id)
            .one(&state.postgres)
            .await?
            .ok_or("missing invocation")?;
        assert_eq!(inv.state, if mode == 2 { "failed" } else { original });
        assert_eq!(
            inv.error_code.as_deref(),
            if mode == 2 {
                Some("settlement_not_executed")
            } else {
                None
            }
        );
        let after = agent_credentials::Entity::find_by_id(credential)
            .one(&state.postgres)
            .await?
            .ok_or("missing credential")?
            .reserved;
        assert_eq!(
            after,
            before + i64::from(mode != 2),
            "release requires finalized unused authorization and must occur once"
        );
    }
    Ok(())
}
async fn pending(
    state: &AppState,
    credential: Uuid,
    capability: Uuid,
    status: &str,
) -> Result<Uuid> {
    let profile = &state.settings.runtime.payment;
    let id = Uuid::new_v4();
    let nonce = format!("0x{:0>64}", id.simple().to_string());
    let payer = "0x1111111111111111111111111111111111111111";
    let requirements = json!({"scheme":"exact","network":profile.network(),"amount":"1","asset":profile.asset,"payTo":payer,"maxTimeoutSeconds":300,"extra":{}});
    let resource =
        json!({"url":"hive://fixture","description":"fixture","mimeType":"application/json"});
    let payload = json!({"x402Version":2,"accepted":requirements,"resource":resource,"payload":{"signature":format!("0x{}","00".repeat(65)),"authorization":{"from":payer,"to":payer,"value":"1","validAfter":"0","validBefore":"200","nonce":nonce}}});
    agent_credentials::Entity::update_many()
        .col_expr(
            agent_credentials::Column::Reserved,
            Expr::col(agent_credentials::Column::Reserved).add(1_i64),
        )
        .filter(agent_credentials::Column::Id.eq(credential))
        .exec(&state.postgres)
        .await?;
    invocations::ActiveModel {
        id: Set(id),
        credential_id: Set(credential),
        capability_id: Set(capability),
        creator_wallet: Set(payer.into()),
        idempotency_key: Set(Uuid::new_v4()),
        request_hash: Set(vec![1; 32]),
        input: Set(json!({})),
        requirements: Set(requirements),
        resource: Set(resource),
        state: Set(status.into()),
        price: Set(1),
        expires_at: Set(200),
        created_at: Set(0),
        updated_at: Set(0),
        result: Set(Some(json!({"ok":true}))),
        payment_payload: Set(Some(payload)),
        payment_method: Set(Some("x402".into())),
        payment_nonce: Set(Some(nonce)),
        settlement_block: Set(Some("0x1".into())),
        ..Default::default()
    }
    .insert(&state.postgres)
    .await?;
    Ok(id)
}

pub(super) async fn checkout_queue(state: &AppState, id: Uuid, credential: Uuid) -> Result {
    let before = agent_credentials::Entity::find_by_id(credential)
        .one(&state.postgres)
        .await?
        .ok_or("missing credential")?
        .reserved;
    pollen::dodo::queue_stale_checkouts(state).await?;
    assert_eq!(
        invocations::Entity::find_by_id(id)
            .one(&state.postgres)
            .await?
            .ok_or("missing invocation")?
            .state,
        "checkout_creating"
    );
    invocations::Entity::update_many()
        .col_expr(
            invocations::Column::UpdatedAt,
            Expr::val(
                hiveway::features::common::model::now()
                    - state.settings.runtime.worker.stale_after_seconds
                    - 1,
            ),
        )
        .filter(invocations::Column::Id.eq(id))
        .exec(&state.postgres)
        .await?;
    pollen::dodo::queue_stale_checkouts(state).await?;
    pollen::dodo::queue_stale_checkouts(state).await?;
    let inv = invocations::Entity::find_by_id(id)
        .one(&state.postgres)
        .await?
        .ok_or("missing invocation")?;
    assert_eq!(inv.state, "checkout_review");
    assert_eq!(
        inv.payment_issue.as_deref(),
        Some("operator_review_required")
    );
    assert_eq!(
        agent_credentials::Entity::find_by_id(credential)
            .one(&state.postgres)
            .await?
            .ok_or("missing credential")?
            .reserved,
        before
    );
    Ok(())
}
