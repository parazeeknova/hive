use super::{
    AgentCredentialsColumn, AgentCredentialsEntity, Alias, ApiError, ApiResult, EntityTrait, Expr,
    ExprTrait, QueryFilter, Tx, Uuid, now,
};
pub(super) async fn release_expired(tx: &Tx, credential: Uuid) -> ApiResult<()> {
    use crate::entities::invocations::{Column as InvocationColumn, Entity as Invocations};
    use sea_orm::{ColumnTrait, QuerySelect};
    let condition = InvocationColumn::CredentialId
        .eq(credential)
        .and(InvocationColumn::State.eq("quoted"))
        .and(InvocationColumn::ExpiresAt.lte(now()));
    let released = Invocations::find()
        .select_only()
        .column_as(
            Expr::col(InvocationColumn::Price)
                .sum()
                .cast_as(Alias::new("bigint")),
            "amount",
        )
        .filter(condition.clone())
        .into_tuple::<Option<i64>>()
        .one(tx)
        .await?
        .flatten()
        .unwrap_or(0);
    {
        Invocations::update_many()
            .col_expr(InvocationColumn::State, Expr::val("expired"))
            .col_expr(InvocationColumn::ErrorCode, Expr::val("quote_expired"))
            .col_expr(InvocationColumn::UpdatedAt, Expr::val(now()))
            .filter(condition)
            .exec(tx)
            .await?;
        AgentCredentialsEntity::update_many()
            .col_expr(
                AgentCredentialsColumn::Reserved,
                Expr::col(AgentCredentialsColumn::Reserved).sub(released),
            )
            .filter(AgentCredentialsColumn::Id.eq(credential))
            .exec(tx)
            .await?;
    }
    Ok(())
}

pub(super) async fn reserve(tx: &Tx, credential: Uuid, price: i64) -> ApiResult<()> {
    let reserved = AgentCredentialsEntity::update_many()
        .col_expr(
            AgentCredentialsColumn::Reserved,
            Expr::col(AgentCredentialsColumn::Reserved).add(Expr::val(price)),
        )
        .filter(
            Expr::col(AgentCredentialsColumn::Id)
                .eq(Expr::val(credential))
                .and(Expr::val(price).lte(Expr::col(AgentCredentialsColumn::PerCallLimit)))
                .and(
                    Expr::col(AgentCredentialsColumn::Spent)
                        .add(Expr::col(AgentCredentialsColumn::Reserved))
                        .add(Expr::val(price))
                        .lte(Expr::col(AgentCredentialsColumn::TotalLimit)),
                ),
        )
        .exec(tx)
        .await?
        .rows_affected;
    if reserved != 1 {
        return Err(ApiError::forbidden());
    }
    Ok(())
}
