//! Durable recovery and bounded workers for read-only capability execution.
use super::{dodo, invocation, reconcile_inner};
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
use sea_orm::{ConnectionTrait, DbBackend, FromQueryResult, Statement, TransactionTrait};
use serde_json::json;
use std::time::Instant;

fn statement(sql: &str, values: Vec<sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}
async fn execute_paid(state: &AppState, inv: Invocation) -> ApiResult<()> {
    let lease = now();
    let claimed=state.postgres.execute_raw(statement("UPDATE invocations SET state='executing',started_at=$1,worker_started_at=$1,updated_at=$1 WHERE id=$2 AND state='paid'",vec![lease.into(),inv.id.into()])).await?.rows_affected();
    if claimed != 1 {
        return Ok(());
    }
    let start = Instant::now();
    let input: RepositoryInput = serde_json::from_value(inv.input.clone())?;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(60),
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
            state.postgres.execute_raw(statement("UPDATE invocations SET state='execution_failed',error_code=$1,payment_issue='refund_required',completed_at=$2,updated_at=$2 WHERE id=$3 AND state='executing' AND payment_method='dodo' AND worker_started_at=$4",vec![code.into(),now().into(),inv.id.into(),lease.into()])).await?;
            return Ok(());
        }
    };
    let elapsed = i64::try_from(start.elapsed().as_millis()).unwrap_or(i64::MAX);
    let bytes = i64::try_from(serde_json::to_vec(&result)?.len()).unwrap_or(i64::MAX);
    let payment_id = inv
        .provider_payment_id
        .as_deref()
        .ok_or_else(|| ApiError::bad("missing_provider_payment"))?;
    let tx = state.postgres.begin().await?;
    let credential = tx
        .query_one_raw(statement(
            "SELECT wallet FROM agent_credentials WHERE id=$1 FOR UPDATE",
            vec![inv.credential_id.into()],
        ))
        .await?
        .ok_or_else(|| ApiError::bad("missing_credential"))?;
    let wallet: String = credential.try_get("", "wallet")?;
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
    tx.execute_raw(statement("UPDATE invocations SET state='settled',result=$1,settlement=$2,execution_ms=$3,result_bytes=$4,completed_at=$5,updated_at=$5 WHERE id=$6",vec![result.into(),serde_json::to_value(settlement)?.into(),elapsed.into(),bytes.into(),now().into(),inv.id.into()])).await?;
    let creator: String = tx
        .query_one_raw(statement(
            "SELECT creator_wallet FROM invocations WHERE id=$1",
            vec![inv.id.into()],
        ))
        .await?
        .ok_or_else(|| ApiError::bad("missing_creator"))?
        .try_get("", "creator_wallet")?;
    tx.execute_raw(statement("INSERT INTO creator_sales(invocation_id,creator_wallet,payer_wallet,amount,network,transaction_hash,created_at) VALUES($1,$2,$3,$4,'dodo:test',$5,$6) ON CONFLICT(invocation_id) DO NOTHING",vec![inv.id.into(),creator.into(),wallet.into(),inv.price.into(),payment_id.into(),now().into()])).await?;
    tx.commit().await?;
    tracing::info!(invocation_id=%inv.id,execution_ms=elapsed,result_bytes=bytes,"Capability delivered after Dodo payment");
    Ok(())
}
/// Perform one recovery cycle. Workers never release uncertain settlements.
/// # Errors
/// Returns database errors; individual provider failures are logged and retried.
pub async fn tick(state: &AppState) -> ApiResult<()> {
    if state.settings.dodo_api_key.is_some()
        && let Err(error) = dodo::process_events(state).await
    {
        tracing::warn!(error = error.code, "Dodo webhook processing pending");
    }
    // Recover read-only Dodo executions after a crash. Payment was already booked.
    state.postgres.execute_raw(statement("UPDATE invocations SET state='paid',updated_at=$1 WHERE payment_method='dodo' AND state='executing' AND updated_at<$2",vec![now().into(),(now()-180).into()])).await?;
    let rows=Invocation::find_by_statement(statement("SELECT * FROM invocations WHERE state IN ('paid','settling','settlement_unknown','checkout_pending') ORDER BY CASE WHEN state='paid' THEN 0 ELSE 1 END,COALESCE(worker_started_at,0) LIMIT 20",vec![])).all(&state.postgres).await?;
    for inv in rows {
        if inv.state != "paid" {
            state
                .postgres
                .execute_raw(statement(
                    "UPDATE invocations SET worker_started_at=$1 WHERE id=$2",
                    vec![now().into(), inv.id.into()],
                ))
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
    // Release abandoned pre-settlement x402 executions only after the request's
    // bounded execution window. A state CAS in the request prevents later settlement.
    let abandoned=Invocation::find_by_statement(statement("SELECT * FROM invocations WHERE payment_method='x402' AND state IN ('verifying','executing') AND updated_at<$1 LIMIT 20",vec![(now()-180).into()])).all(&state.postgres).await?;
    for inv in abandoned {
        super::fail(state, inv.id, inv.credential_id, "execution_interrupted").await?;
    }
    tracing::debug!(
        activity = json!({"worker":"pollen"}).to_string(),
        "Recovery cycle complete"
    );
    Ok(())
}
