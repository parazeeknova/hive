use crate::entities::refund_operations::{Column, Entity};
use sea_orm::sea_query::{Expr, ExprTrait};
use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut table =
            sea_orm::Schema::new(sea_orm::DbBackend::Postgres).create_table_from_entity(Entity);
        table.check((
            "refund_amount_positive",
            Expr::col(Column::Amount).gt(0_i64),
        ));
        table.check((
            "refund_whole_cents",
            Expr::col(Column::Amount).modulo(10_000_i64).eq(0_i64),
        ));
        table.check((
            "refund_state",
            Expr::col(Column::State).is_in([
                "prepared",
                "creating",
                "pending",
                "review",
                "succeeded",
                "failed",
            ]),
        ));
        table.check((
            "refund_success_has_id",
            Expr::col(Column::State)
                .ne("succeeded")
                .or(Expr::col(Column::RefundId).is_not_null()),
        ));
        manager.create_table(table).await?;
        manager
            .create_index(
                Index::create()
                    .name("refund_recovery_activity")
                    .table(Entity)
                    .col(Column::State)
                    .col(Column::UpdatedAt)
                    .to_owned(),
            )
            .await
    }
    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "Refund history requires forward migrations".into(),
        ))
    }
}
