//! Serialize refund state and budget transitions with the same credential lock as payments.
use crate::{
    entities::{agent_credentials, invocations, refund_operations},
    features::{
        common::{
            error::{ApiError, ApiResult},
            model::now,
        },
        pollen::invocation,
    },
    state::AppState,
};
use sea_orm::sea_query::{Expr, ExprTrait};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, TransactionTrait};
use uuid::Uuid;

pub(super) async fn transition(
    state: &AppState,
    id: Uuid,
    next: &str,
    refund_id: Option<&str>,
    error: Option<&str>,
) -> ApiResult<()> {
    let tx = state.postgres.begin().await?;
    let original = invocations::Entity::find_by_id(id)
        .one(&tx)
        .await?
        .ok_or_else(|| ApiError::bad("invocation_not_found"))?;
    crate::database::lock_credential(&tx, original.credential_id).await?;
    invocation(&tx, id, original.credential_id).await?;
    let current = refund_operations::Entity::find_by_id(id)
        .one(&tx)
        .await?
        .ok_or_else(|| ApiError::bad("refund_not_found"))?;
    if !matches!(current.state.as_str(), "succeeded" | "failed") {
        refund_operations::Entity::update_many()
            .col_expr(refund_operations::Column::State, Expr::val(next))
            .col_expr(
                refund_operations::Column::RefundId,
                Expr::val(refund_id.map(str::to_owned).or(current.refund_id)),
            )
            .col_expr(refund_operations::Column::ErrorCode, Expr::val(error))
            .col_expr(refund_operations::Column::UpdatedAt, Expr::val(now()))
            .filter(refund_operations::Column::InvocationId.eq(id))
            .exec(&tx)
            .await?;
        invocations::Entity::update_many()
            .col_expr(
                invocations::Column::PaymentIssue,
                Expr::val(match next {
                    "failed" => "refund_rejected",
                    "pending" => "refund_pending",
                    _ => "refund_review_required",
                }),
            )
            .filter(invocations::Column::Id.eq(id))
            .exec(&tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}
pub(super) async fn book(
    state: &AppState,
    op: &refund_operations::Model,
    refund_id: &str,
) -> ApiResult<()> {
    let tx = state.postgres.begin().await?;
    let original = invocations::Entity::find_by_id(op.invocation_id)
        .one(&tx)
        .await?
        .ok_or_else(|| ApiError::bad("invocation_not_found"))?;
    crate::database::lock_credential(&tx, original.credential_id).await?;
    let inv = invocation(&tx, original.id, original.credential_id).await?;
    let current = refund_operations::Entity::find_by_id(op.invocation_id)
        .one(&tx)
        .await?
        .ok_or_else(|| ApiError::bad("refund_not_found"))?;
    if current.state != "succeeded" {
        if inv.state != "execution_failed"
            || inv.payment_method.as_deref() != Some("dodo")
            || inv.provider_payment_id.as_deref() != Some(op.payment_id.as_str())
            || inv.price != op.amount
        {
            return Err(ApiError::bad("refund_invocation_binding_mismatch"));
        }
        refund_operations::Entity::update_many()
            .col_expr(refund_operations::Column::State, Expr::val("succeeded"))
            .col_expr(refund_operations::Column::RefundId, Expr::val(refund_id))
            .col_expr(
                refund_operations::Column::ErrorCode,
                Expr::val(Option::<String>::None),
            )
            .col_expr(refund_operations::Column::UpdatedAt, Expr::val(now()))
            .filter(refund_operations::Column::InvocationId.eq(op.invocation_id))
            .exec(&tx)
            .await?;
        agent_credentials::Entity::update_many()
            .col_expr(
                agent_credentials::Column::Spent,
                Expr::col(agent_credentials::Column::Spent).sub(op.amount),
            )
            .filter(agent_credentials::Column::Id.eq(inv.credential_id))
            .exec(&tx)
            .await?;
        invocations::Entity::update_many()
            .col_expr(invocations::Column::PaymentIssue, Expr::val("refunded"))
            .filter(invocations::Column::Id.eq(inv.id))
            .exec(&tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}
