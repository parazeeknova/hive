use super::{
    ApiError, ApiResult, ConnectionTrait, EntityTrait, Expr, ExprTrait, Invocation,
    InvocationsColumn, InvocationsEntity, QueryFilter, QuoteResponse, SqlQuery, StatusCode, Uuid,
    required,
};
pub(super) async fn existing_quote(
    tx: &impl ConnectionTrait,
    credential: Uuid,
    key: Uuid,
    fingerprint: &[u8],
) -> ApiResult<Option<QuoteResponse>> {
    let existing = InvocationsEntity::find()
        .filter(
            Expr::col(InvocationsColumn::CredentialId)
                .eq(Expr::val(credential))
                .and(Expr::col(InvocationsColumn::IdempotencyKey).eq(Expr::val(key))),
        )
        .into_model::<Invocation>()
        .one(tx)
        .await?;
    if let Some(inv) = existing {
        let original: Vec<u8> = tx
            .query_one(
                &SqlQuery::select()
                    .from(InvocationsEntity)
                    .column(InvocationsColumn::RequestHash)
                    .and_where(Expr::col(InvocationsColumn::Id).eq(Expr::val(inv.id)))
                    .to_owned(),
            )
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound("required row missing".into()))?
            .try_get_by_index(0)?;
        if original != fingerprint {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "idempotency_key_reused_with_different_input",
            ));
        }
        let result = QuoteResponse {
            id: inv.id,
            expires_at: inv.expires_at,
            payment: required(&inv)?,
        };
        return Ok(Some(result));
    }
    Ok(None)
}
