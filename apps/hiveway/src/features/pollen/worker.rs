//! Durable recovery and bounded workers for read-only capability execution.
use super::{dodo, invocation, reconcile_inner};
use crate::entities::creator_sales::Column as CreatorSalesColumn;
use crate::entities::creator_sales::Entity as CreatorSalesEntity;
use crate::entities::invocations::Column as InvocationsColumn;
use crate::entities::invocations::Entity as InvocationsEntity;
use crate::{
    features::{
        cell,
        common::{
            error::{ApiError, ApiResult},
            model::{Invocation, RepositoryInput, Settlement, now},
        },
    },
    state::AppState,
};
use sea_orm::sea_query::{Expr, ExprTrait, Func, OnConflict, Order, Query as SqlQuery};
use sea_orm::{
    ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};
use serde_json::json;
use std::time::Instant;

async fn execute_paid(state: &AppState, inv: Invocation) -> ApiResult<()> {
    let lease = now();
    let claimed = InvocationsEntity::update_many()
        .col_expr(InvocationsColumn::State, Expr::val("executing"))
        .col_expr(InvocationsColumn::StartedAt, Expr::val(lease))
        .col_expr(InvocationsColumn::WorkerStartedAt, Expr::val(lease))
        .col_expr(InvocationsColumn::UpdatedAt, Expr::val(lease))
        .filter(
            Expr::col(InvocationsColumn::Id)
                .eq(Expr::val(inv.id))
                .and(Expr::col(InvocationsColumn::State).eq(Expr::val("paid"))),
        )
        .exec(&state.postgres)
        .await?
        .rows_affected;
    if claimed != 1 {
        return Ok(());
    }
    let start = Instant::now();
    let input: RepositoryInput = serde_json::from_value(inv.input.clone())?;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(state.settings.runtime.worker.execution_timeout_seconds),
        cell::execute_capability(state, inv.capability_id, &input),
    )
    .await;
    let result = match result {
        Ok(Ok(output)) => output,
        other => {
            let code = match other {
                Ok(Err(error)) => error.code,
                _ => "execution_timeout",
            };
            InvocationsEntity::update_many()
                .col_expr(InvocationsColumn::State, Expr::val("execution_failed"))
                .col_expr(InvocationsColumn::ErrorCode, Expr::val(code))
                .col_expr(
                    InvocationsColumn::PaymentIssue,
                    Expr::val("refund_required"),
                )
                .col_expr(InvocationsColumn::CompletedAt, Expr::val(now()))
                .col_expr(InvocationsColumn::UpdatedAt, Expr::val(now()))
                .filter(
                    Expr::col(InvocationsColumn::Id)
                        .eq(Expr::val(inv.id))
                        .and(Expr::col(InvocationsColumn::State).eq(Expr::val("executing")))
                        .and(Expr::col(InvocationsColumn::PaymentMethod).eq(Expr::val("dodo")))
                        .and(Expr::col(InvocationsColumn::WorkerStartedAt).eq(Expr::val(lease))),
                )
                .exec(&state.postgres)
                .await?;
            return Ok(());
        }
    };
    complete_paid(state, inv, lease, start, result).await
}
async fn complete_paid(
    state: &AppState,
    inv: Invocation,
    lease: i64,
    start: Instant,
    result: serde_json::Value,
) -> ApiResult<()> {
    let elapsed = i64::try_from(start.elapsed().as_millis()).unwrap_or(i64::MAX);
    let bytes = i64::try_from(serde_json::to_vec(&result)?.len()).unwrap_or(i64::MAX);
    let payment_id = inv
        .provider_payment_id
        .as_deref()
        .ok_or_else(|| ApiError::bad("missing_provider_payment"))?;
    let tx = state.postgres.begin().await?;
    let wallet = crate::database::lock_credential(&tx, inv.credential_id)
        .await?
        .wallet;
    let current = invocation(&tx, inv.id, inv.credential_id).await?;
    if current.state != "executing" || current.worker_started_at != Some(lease) {
        tx.commit().await?;
        return Ok(());
    }
    let settlement = Settlement {
        success: true,
        transaction: payment_id.into(),
        network: "dodo:test".into(),
        payer: Some(wallet.clone()),
        error_reason: None,
    };
    InvocationsEntity::update_many()
        .col_expr(InvocationsColumn::State, Expr::val("settled"))
        .col_expr(InvocationsColumn::Result, Expr::val((result).clone()))
        .col_expr(
            InvocationsColumn::Settlement,
            Expr::val((serde_json::to_value(settlement)?).clone()),
        )
        .col_expr(InvocationsColumn::ExecutionMs, Expr::val(elapsed))
        .col_expr(InvocationsColumn::ResultBytes, Expr::val(bytes))
        .col_expr(InvocationsColumn::CompletedAt, Expr::val(now()))
        .col_expr(InvocationsColumn::UpdatedAt, Expr::val(now()))
        .filter(Expr::col(InvocationsColumn::Id).eq(Expr::val(inv.id)))
        .exec(&tx)
        .await?;
    let creator: String = tx
        .query_one(
            &SqlQuery::select()
                .from(InvocationsEntity)
                .column(InvocationsColumn::CreatorWallet)
                .and_where(Expr::col(InvocationsColumn::Id).eq(Expr::val(inv.id)))
                .to_owned(),
        )
        .await?
        .ok_or_else(|| ApiError::bad("missing_creator"))?
        .try_get("", "creator_wallet")?;
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
                Expr::val(inv.id),
                Expr::val((creator).clone()),
                Expr::val((wallet).clone()),
                Expr::val(inv.price),
                Expr::val("dodo:test"),
                Expr::val(payment_id),
                Expr::val(now()),
            ])
            .on_conflict(
                OnConflict::columns([CreatorSalesColumn::InvocationId])
                    .do_nothing()
                    .to_owned(),
            )
            .to_owned(),
    )
    .await?;
    tx.commit().await?;
    tracing::info!(invocation_id=%inv.id,execution_ms=elapsed,result_bytes=bytes,"Capability delivered after Dodo payment");
    Ok(())
}
/// Perform one recovery cycle. Workers never release uncertain settlements.
/// # Errors
/// Returns database errors; individual provider failures are logged and retried.
pub async fn tick(state: &AppState) -> ApiResult<()> {
    crate::features::identity::cleanup_challenges(state).await?;
    dodo::queue_stale_checkouts(state).await?;
    if state.settings.dodo_api_key.is_some()
        && let Err(error) = dodo::refunds::tick(state).await
    {
        tracing::warn!(error = error.code, "Refund recovery pending");
    }
    if state.settings.dodo_api_key.is_some()
        && let Err(error) = dodo::process_events(state).await
    {
        tracing::warn!(error = error.code, "Dodo webhook processing pending");
    }
    // Recover read-only Dodo executions after a crash. Payment was already booked.
    InvocationsEntity::update_many()
        .col_expr(InvocationsColumn::State, Expr::val("paid"))
        .col_expr(InvocationsColumn::UpdatedAt, Expr::val(now()))
        .filter(
            Expr::col(InvocationsColumn::PaymentMethod)
                .eq(Expr::val("dodo"))
                .and(Expr::col(InvocationsColumn::State).eq(Expr::val("executing")))
                .and(Expr::col(InvocationsColumn::UpdatedAt).lt(Expr::val(
                    now() - state.settings.runtime.worker.stale_after_seconds,
                ))),
        )
        .exec(&state.postgres)
        .await?;
    let rows = InvocationsEntity::find()
        .filter(Expr::col(InvocationsColumn::State).is_in([
            "paid",
            "settling",
            "settlement_unknown",
            "checkout_pending",
        ]))
        .order_by(
            Expr::expr(
                Expr::case(
                    Expr::col(InvocationsColumn::State).eq(Expr::val("paid")),
                    Expr::val(0_i64),
                )
                .finally(Expr::val(1_i64)),
            ),
            Order::Asc,
        )
        .order_by(
            Expr::expr(Func::coalesce([
                Expr::col(InvocationsColumn::WorkerStartedAt),
                Expr::val(0_i64),
            ])),
            Order::Asc,
        )
        .limit(crate::database::limit(
            state.settings.runtime.worker.batch_size,
        )?)
        .into_model::<Invocation>()
        .all(&state.postgres)
        .await?;
    for inv in rows {
        if inv.state != "paid" {
            InvocationsEntity::update_many()
                .col_expr(InvocationsColumn::WorkerStartedAt, Expr::val(now()))
                .filter(
                    Expr::col(InvocationsColumn::Id)
                        .eq(Expr::val(inv.id))
                        .and(Expr::col(InvocationsColumn::State).eq(Expr::val(inv.state.clone()))),
                )
                .exec(&state.postgres)
                .await?;
        }

        let result = if inv.state == "paid" {
            execute_paid(state, inv).await
        } else if inv.payment_method.as_deref() == Some("dodo") {
            dodo::reconcile(state, inv.id, inv.credential_id)
                .await
                .map(|_| ())
        } else {
            reconcile_inner(state, inv.id, inv.credential_id)
                .await
                .map(|_| ())
        };
        if let Err(error) = result {
            tracing::warn!(error = error.code, "Payment recovery pending");
        }
    }
    release_abandoned(state).await
}

async fn release_abandoned(state: &AppState) -> ApiResult<()> {
    // Release abandoned pre-settlement x402 executions only after the request's
    // bounded execution window. A state CAS in the request prevents later settlement.
    let abandoned = InvocationsEntity::find()
        .filter(
            Expr::col(InvocationsColumn::PaymentMethod)
                .eq(Expr::val("x402"))
                .and(Expr::col(InvocationsColumn::State).is_in(["verifying", "executing"]))
                .and(Expr::col(InvocationsColumn::UpdatedAt).lt(Expr::val(
                    now() - state.settings.runtime.worker.stale_after_seconds,
                ))),
        )
        .limit(crate::database::limit(
            state.settings.runtime.worker.batch_size,
        )?)
        .into_model::<Invocation>()
        .all(&state.postgres)
        .await?;
    for inv in abandoned {
        super::fail(state, inv.id, inv.credential_id, "execution_interrupted").await?;
    }
    tracing::debug!(
        activity = json!({"worker":"pollen"}).to_string(),
        "Recovery cycle complete"
    );
    Ok(())
}
