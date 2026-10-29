use super::super::provider::request;
use super::{identifier, ledger, load};
use crate::{
    entities::refund_operations,
    features::common::error::{ApiError, ApiResult},
    state::AppState,
};
use serde_json::Value;
use uuid::Uuid;

pub(super) async fn reconcile(state: &AppState, id: Uuid) -> ApiResult<()> {
    let op = load(state, id).await?;
    if op.state == "succeeded" {
        return Ok(());
    }
    let payment = request(
        state,
        reqwest::Method::GET,
        &format!("/payments/{}", identifier(&op.payment_id)?),
        None,
    )
    .await?;
    if payment["payment_id"] != op.payment_id
        || payment["metadata"]["hive_invocation_id"] != id.to_string()
    {
        return Err(ApiError::bad("refund_payment_binding_mismatch"));
    }
    let refund_id = op.refund_id.clone().or_else(|| {
        payment["refunds"].as_array()?.iter().find(|refund| {
            refund["payment_id"] == op.payment_id
                && refund["is_partial"] == false
                && refund["amount"].as_i64() == Some(op.amount / 10_000)
                && refund["currency"] == "USD"
        })?["refund_id"]
            .as_str()
            .map(str::to_owned)
    });
    let Some(refund_id) = refund_id else {
        ledger::transition(
            state,
            id,
            "review",
            None,
            Some("refund_creation_unconfirmed"),
        )
        .await?;
        return Ok(());
    };
    identifier(&refund_id)?;
    let refund = request(
        state,
        reqwest::Method::GET,
        &format!("/refunds/{refund_id}"),
        None,
    )
    .await?;
    if !bound_refund(&refund, &op, &refund_id) {
        return Err(ApiError::bad("refund_binding_mismatch"));
    }
    if refund["status"] == "succeeded" {
        ledger::book(state, &op, &refund_id).await?;
    } else {
        let next = if refund["status"] == "failed" {
            "failed"
        } else {
            "pending"
        };
        ledger::transition(state, id, next, Some(&refund_id), None).await?;
    }
    Ok(())
}
fn bound_refund(refund: &Value, op: &refund_operations::Model, id: &str) -> bool {
    refund["refund_id"] == id
        && refund["payment_id"] == op.payment_id
        && refund["is_partial"] == false
        && refund["currency"] == "USD"
        && refund["amount"].as_i64() == Some(op.amount / 10_000)
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn partial_or_mismatched_refunds_cannot_restore_budget() {
        let op = refund_operations::Model {
            invocation_id: Uuid::nil(),
            payment_id: "pay_1".into(),
            refund_id: None,
            state: "pending".into(),
            amount: 10_000,
            error_code: None,
            created_at: 0,
            updated_at: 0,
        };
        let valid = json!({"refund_id":"ref_1","payment_id":"pay_1","is_partial":false,"amount":1,"currency":"USD"});
        assert!(bound_refund(&valid, &op, "ref_1"));
        for (field, value) in [
            ("amount", json!(2)),
            ("currency", json!("EUR")),
            ("is_partial", json!(true)),
            ("payment_id", json!("other")),
        ] {
            let mut altered = valid.clone();
            altered[field] = value;
            assert!(!bound_refund(&altered, &op, "ref_1"));
        }
    }
}
