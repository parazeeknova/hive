use crate::entities::creator_earnings::{Column, Entity};
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
            "earnings_amount_positive",
            Expr::col(Column::Amount).gt(0_i64),
        ));
        table.check((
            "earnings_payment_method",
            Expr::col(Column::PaymentMethod).is_in(["x402", "dodo"]),
        ));
        manager.create_table(table).await?;
        manager
            .create_index(
                Index::create()
                    .name("creator_earnings_wallet_activity")
                    .table(Entity)
                    .col(Column::CreatorWallet)
                    .col((Column::CreatedAt, IndexOrder::Desc))
                    .col((Column::InvocationId, IndexOrder::Desc))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("creator_earnings_capability_activity")
                    .table(Entity)
                    .col(Column::CapabilityId)
                    .col((Column::CreatedAt, IndexOrder::Desc))
                    .to_owned(),
            )
            .await?;
        // Ledger-only backfill: every settled sale already recorded becomes one
        // credit. No custody, no balances held — this is analytics history.
        manager
            .get_connection()
            .execute_unprepared(
                "INSERT INTO creator_earnings (invocation_id, capability_id, creator_wallet, payer_wallet, amount, network, transaction_hash, payment_method, refunded, created_at) \
                 SELECT s.invocation_id, i.capability_id, s.creator_wallet, s.payer_wallet, s.amount, s.network, s.transaction_hash, \
                 COALESCE(i.payment_method, CASE WHEN s.network = 'dodo:test' THEN 'dodo' ELSE 'x402' END), false, s.created_at \
                 FROM creator_sales s JOIN invocations i ON i.id = s.invocation_id ON CONFLICT DO NOTHING",
            )
            .await?;
        Ok(())
    }
    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "Earnings history requires forward migrations".into(),
        ))
    }
}
