use super::super::provider::{identifier, request};
use super::{ledger, load, reconciliation};
use crate::{
    entities::refund_operations,
    features::common::{
        error::{ApiError, ApiResult},
        model::now,
    },
    state::AppState,
};
use sea_orm::sea_query::Expr;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::json;
use uuid::Uuid;
pub(super) async fn send(state: &AppState, id: Uuid) -> ApiResult<()> {
    let op = load(state, id).await?;
    if op.state != "prepared" {
        return Ok(());
    }
    let payment = &op.payment_id;
    let canonical = request(
        state,
        reqwest::Method::GET,
        &format!("/payments/{}", identifier(payment)?),
        None,
    )
    .await?;
    if canonical["payment_id"] != *payment
        || canonical["metadata"]["hive_invocation_id"] != id.to_string()
        || canonical["total_amount"].as_i64() != Some(op.amount / 10_000)
        || canonical["currency"] != "USD"
    {
        return Err(ApiError::bad("refund_payment_binding_mismatch"));
    }
    let refunds = canonical["refunds"]
        .as_array()
        .ok_or_else(|| ApiError::unavailable("missing_refund_history"))?;
    if !refunds.is_empty() {
        return reconciliation::reconcile(state, id).await;
    }
    let claimed = refund_operations::Entity::update_many()
        .col_expr(refund_operations::Column::State, Expr::val("creating"))
        .col_expr(refund_operations::Column::UpdatedAt, Expr::val(now()))
        .filter(refund_operations::Column::InvocationId.eq(id))
        .filter(refund_operations::Column::State.eq("prepared"))
        .exec(&state.postgres)
        .await?
        .rows_affected;
    if claimed != 1 {
        return Ok(());
    }
    // The durable creation claim precedes the one and only provider POST.
    let result = request(state, reqwest::Method::POST, "/refunds", Some(json!({"payment_id":payment,"reason":"Hiveway capability execution failed","metadata":{"hive_invocation_id":id.to_string()}}))).await;
    let (refund_id, issue) = match result {
        Ok(value) => (
            value["refund_id"]
                .as_str()
                .filter(|value| identifier(value).is_ok())
                .map(str::to_owned),
            None,
        ),
        Err(error) => (None, Some(error.code)),
    };
    let next = if issue == Some("dodo_request_rejected") {
        "failed"
    } else if refund_id.is_some() {
        "pending"
    } else {
        "review"
    };
    ledger::transition(state, id, next, refund_id.as_deref(), issue).await?;
    Ok(())
}
