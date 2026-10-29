//! Durable, at-most-once refund creation. Ambiguous creation is reconciled, never reposted.
mod dispatch;
mod ledger;
mod reconciliation;
use super::provider::identifier;
use crate::{
    entities::{invocations, refund_operations},
    features::{
        common::{
            error::{ApiError, ApiResult},
            model::now,
        },
        identity,
        pollen::invocation,
    },
    state::AppState,
};
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    TransactionTrait,
};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, utoipa::ToSchema)]
pub struct RefundStatus {
    pub invocation_id: Uuid,
    pub state: String,
    pub amount: String,
    pub refund_id: Option<String>,
    pub error_code: Option<String>,
}
impl From<refund_operations::Model> for RefundStatus {
    fn from(value: refund_operations::Model) -> Self {
        Self {
            invocation_id: value.invocation_id,
            state: value.state,
            amount: value.amount.to_string(),
            refund_id: value.refund_id,
            error_code: value.error_code,
        }
    }
}
/// Request or reconcile the full refund for this agent's failed paid execution.
/// # Errors
/// Rejects unauthorized, unpaid, or successful invocations and provider/database failures.
#[utoipa::path(post,path="/v1/invocations/{id}/refund",params(("id"=Uuid,Path)),responses((status=200,body=RefundStatus)))]
pub async fn refund(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<RefundStatus>> {
    let credential = identity::authenticate(&state, &headers, "receipts:read").await?;
    ensure(&state, id, credential.id).await?;
    reconciliation::reconcile(&state, id).await?;
    Ok(Json(load(&state, id).await?.into()))
}
/// Read the refund operation owned by this credential.
/// # Errors
/// Rejects missing operations and unauthorized credentials.
#[utoipa::path(get,path="/v1/invocations/{id}/refund",params(("id"=Uuid,Path)),responses((status=200,body=RefundStatus)))]
pub async fn status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<RefundStatus>> {
    let credential = identity::authenticate(&state, &headers, "receipts:read").await?;
    if invocations::Entity::find_by_id(id)
        .filter(invocations::Column::CredentialId.eq(credential.id))
        .one(&state.postgres)
        .await?
        .is_none()
    {
        return Err(ApiError::new(
            axum::http::StatusCode::NOT_FOUND,
            "invocation_not_found",
        ));
    }
    Ok(Json(load(&state, id).await?.into()))
}
async fn load(state: &AppState, id: Uuid) -> ApiResult<refund_operations::Model> {
    refund_operations::Entity::find_by_id(id)
        .one(&state.postgres)
        .await?
        .ok_or_else(|| ApiError::new(axum::http::StatusCode::NOT_FOUND, "refund_not_found"))
}
async fn ensure(state: &AppState, id: Uuid, credential: Uuid) -> ApiResult<()> {
    if state.settings.dodo_api_key.is_none() {
        return Err(ApiError::unavailable("dodo_not_configured"));
    }
    let tx = state.postgres.begin().await?;
    crate::database::lock_credential(&tx, credential).await?;
    let inv = invocation(&tx, id, credential).await?;
    if inv.payment_method.as_deref() != Some("dodo")
        || inv.state != "execution_failed"
        || inv.price < 10_000
        || inv.price % 10_000 != 0
    {
        return Err(ApiError::bad("invocation_not_refundable"));
    }
    if refund_operations::Entity::find_by_id(id)
        .one(&tx)
        .await?
        .is_some()
    {
        tx.commit().await?;
        return dispatch::send(state, id).await;
    }
    let payment = inv
        .provider_payment_id
        .ok_or_else(|| ApiError::bad("missing_provider_payment"))?;
    identifier(&payment)?;
    refund_operations::ActiveModel {
        invocation_id: Set(id),
        payment_id: Set(payment.clone()),
        refund_id: Set(None),
        state: Set("prepared".into()),
        amount: Set(inv.price),
        error_code: Set(None),
        created_at: Set(now()),
        updated_at: Set(now()),
    }
    .insert(&tx)
    .await?;
    invocations::Entity::update_many()
        .col_expr(
            invocations::Column::PaymentIssue,
            Expr::val("refund_creating"),
        )
        .filter(invocations::Column::Id.eq(id))
        .exec(&tx)
        .await?;
    tx.commit().await?;
    dispatch::send(state, id).await
}
/// Run bounded creation and reconciliation batches for failed paid executions.
/// # Errors
/// Returns database errors; provider failures stay retryable or queued for review.
pub async fn tick(state: &AppState) -> ApiResult<()> {
    let limit = crate::database::limit(state.settings.runtime.worker.batch_size)?;
    let failed = invocations::Entity::find()
        .filter(invocations::Column::State.eq("execution_failed"))
        .filter(invocations::Column::PaymentMethod.eq("dodo"))
        .filter(invocations::Column::PaymentIssue.eq("refund_required"))
        .order_by_asc(invocations::Column::UpdatedAt)
        .limit(limit)
        .all(&state.postgres)
        .await?;
    for inv in failed {
        if let Err(error) = ensure(state, inv.id, inv.credential_id).await {
            tracing::warn!(invocation_id=%inv.id, error=error.code, "Refund creation pending");
        }
    }
    let operations = refund_operations::Entity::find()
        .filter(
            refund_operations::Column::State.is_in(["prepared", "creating", "pending", "review"]),
        )
        .order_by_asc(refund_operations::Column::UpdatedAt)
        .limit(limit)
        .all(&state.postgres)
        .await?;
    for op in operations {
        refund_operations::Entity::update_many()
            .col_expr(refund_operations::Column::UpdatedAt, Expr::val(now()))
            .filter(refund_operations::Column::InvocationId.eq(op.invocation_id))
            .exec(&state.postgres)
            .await?;
        let result = if op.state == "prepared" {
            dispatch::send(state, op.invocation_id).await
        } else {
            reconciliation::reconcile(state, op.invocation_id).await
        };
        if let Err(error) = result {
            tracing::warn!(invocation_id=%op.invocation_id, error=error.code, "Refund reconciliation pending");
        }
    }
    Ok(())
}
