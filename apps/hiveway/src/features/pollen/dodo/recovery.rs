//! Recover checkout creation only from provider-confirmed payment/session bindings.
use super::{identifier, invocation, matches_payment, reconcile, request};
use crate::{
    entities::invocations::{Column, Entity},
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
    extract::{Path, State},
    http::HeaderMap,
    response::Response,
};
use sea_orm::sea_query::Expr;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, TransactionTrait};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RecoverCheckout {
    pub payment_id: String,
}
/// Recover this credential's uncertain checkout from a canonical provider payment.
/// # Errors
/// Rejects credential, payment binding, state, and provider/database failures.
#[utoipa::path(post,path="/v1/invocations/{id}/checkout/recover",request_body=RecoverCheckout,params(("id"=Uuid,Path)),responses((status=200,description="Recovered checkout receipt")))]
pub async fn recover(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<RecoverCheckout>,
) -> ApiResult<Response> {
    let credential = identity::authenticate(&state, &headers, "receipts:read").await?;
    recover_paid_checkout(&state, id, credential.id, &body.payment_id).await?;
    reconcile(&state, id, credential.id).await
}

pub(super) async fn recover_paid_checkout(
    state: &AppState,
    id: Uuid,
    credential: Uuid,
    payment_id: &str,
) -> ApiResult<()> {
    identifier(payment_id)?;
    let mut inv = Entity::find_by_id(id)
        .filter(Column::CredentialId.eq(credential))
        .into_model::<Invocation>()
        .one(&state.postgres)
        .await?
        .ok_or_else(|| ApiError::new(axum::http::StatusCode::NOT_FOUND, "invocation_not_found"))?;
    if inv.payment_method.as_deref() != Some("dodo")
        || !matches!(inv.state.as_str(), "checkout_creating" | "checkout_review")
    {
        return Ok(());
    }
    let payment = request(
        state,
        reqwest::Method::GET,
        &format!("/payments/{payment_id}"),
        None,
    )
    .await?;
    let session_id = payment["checkout_session_id"]
        .as_str()
        .ok_or_else(|| ApiError::bad("missing_checkout_id"))?;
    identifier(session_id)?;
    let session = request(
        state,
        reqwest::Method::GET,
        &format!("/checkouts/{session_id}"),
        None,
    )
    .await?;
    inv.checkout_id = Some(session_id.into());
    let product = state
        .settings
        .dodo_product_id
        .as_deref()
        .ok_or_else(|| ApiError::unavailable("dodo_product_not_configured"))?;
    if payment["payment_id"] != payment_id || !matches_payment(&payment, &session, &inv, product) {
        return Err(ApiError::bad("dodo_payment_binding_mismatch"));
    }
    let tx = state.postgres.begin().await?;
    let owner = crate::database::lock_credential(&tx, credential).await?;
    if payment["metadata"]["hive_wallet"] != owner.wallet {
        return Err(ApiError::bad("dodo_payment_binding_mismatch"));
    }
    let current = invocation(&tx, id, credential).await?;
    if matches!(
        current.state.as_str(),
        "checkout_creating" | "checkout_review"
    ) {
        Entity::update_many()
            .col_expr(Column::State, Expr::val("checkout_pending"))
            .col_expr(Column::CheckoutId, Expr::val(session_id))
            .col_expr(Column::PaymentIssue, Expr::val(Option::<String>::None))
            .col_expr(Column::UpdatedAt, Expr::val(now()))
            .filter(Column::Id.eq(id))
            .exec(&tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}
