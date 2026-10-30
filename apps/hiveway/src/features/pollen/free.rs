//! Free execution keeps the same durable admission and replay boundaries as paid calls.
use super::{
    ApiError, ApiResult, AppState, EntityTrait, Expr, ExprTrait, Invocation, InvocationsColumn,
    InvocationsEntity, QueryFilter, Response, TransactionTrait, Tx, cell, fail, invocation, now,
    receipt,
};

pub(super) async fn invoke(state: &AppState, tx: Tx, inv: Invocation) -> ApiResult<Response> {
    InvocationsEntity::update_many()
        .col_expr(InvocationsColumn::State, Expr::val("executing"))
        .col_expr(InvocationsColumn::PaymentMethod, Expr::val("free"))
        .col_expr(InvocationsColumn::StartedAt, Expr::val(now()))
        .col_expr(InvocationsColumn::UpdatedAt, Expr::val(now()))
        .filter(Expr::col(InvocationsColumn::Id).eq(inv.id))
        .exec(&tx)
        .await?;
    tx.commit().await?;
    let start = std::time::Instant::now();
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(state.settings.runtime.worker.execution_timeout_seconds),
        cell::execute_capability(state, inv.capability_id, &inv.input),
    )
    .await;
    let output = match output {
        Ok(Ok(output)) => output,
        other => {
            let error = match other {
                Ok(Err(error)) => error,
                _ => ApiError::unavailable("execution_timeout"),
            };
            fail(state, inv.id, inv.credential_id, error.code).await?;
            return Err(error);
        }
    };
    let tx = state.postgres.begin().await?;
    crate::database::lock_credential(&tx, inv.credential_id).await?;
    let current = invocation(&tx, inv.id, inv.credential_id).await?;
    if current.state == "executing" {
        InvocationsEntity::update_many()
            .col_expr(InvocationsColumn::State, Expr::val("settled"))
            .col_expr(
                InvocationsColumn::ResultBytes,
                Expr::val(i64::try_from(serde_json::to_vec(&output)?.len()).unwrap_or(i64::MAX)),
            )
            .col_expr(InvocationsColumn::Result, Expr::val(output))
            .col_expr(
                InvocationsColumn::ExecutionMs,
                Expr::val(i64::try_from(start.elapsed().as_millis()).unwrap_or(i64::MAX)),
            )
            .col_expr(InvocationsColumn::CompletedAt, Expr::val(now()))
            .col_expr(InvocationsColumn::UpdatedAt, Expr::val(now()))
            .filter(Expr::col(InvocationsColumn::Id).eq(inv.id))
            .exec(&tx)
            .await?;
    }
    let response = receipt(invocation(&tx, inv.id, inv.credential_id).await?)?;
    tx.commit().await?;
    Ok(response)
}
