use crate::entities::capabilities::{Column as CapabilityColumn, Entity as CapabilityEntity};
use crate::entities::wasm_artifacts::{Column, Entity};
use sea_orm::sea_query::{Alias, Expr, ExprTrait, Func};
use sea_orm::sea_query::{Check, IntoIden, PostgresQueryBuilder, TableBuilder};
use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut table =
            sea_orm::Schema::new(sea_orm::DbBackend::Postgres).create_table_from_entity(Entity);
        table.check((
            "artifact_digest_length",
            Expr::expr(Func::cust(Alias::new("octet_length")).arg(Expr::col(Column::Digest)))
                .eq(32_i64),
        ));
        table.check((
            "artifact_size",
            Expr::expr(Func::cust(Alias::new("octet_length")).arg(Expr::col(Column::Wasm)))
                .between(8_i64, 1_048_576_i64),
        ));
        manager.create_table(table).await?;
        manager
            .create_index(
                Index::create()
                    .name("artifact_owner_digest")
                    .table(Entity)
                    .col(Column::OwnerWallet)
                    .col(Column::Digest)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(CapabilityEntity)
                    .drop_constraint("capabilities_rule_2")
                    .to_owned(),
            )
            .await?;
        let mut sql = String::from("ALTER TABLE ");
        PostgresQueryBuilder.prepare_iden(&CapabilityEntity.into_iden(), &mut sql);
        sql.push_str(" ADD ");
        PostgresQueryBuilder.prepare_check_constraint(
            &Check::from((
                "capabilities_rule_2",
                Expr::col(CapabilityColumn::Runtime).is_in([
                    "repository_stack",
                    "dependency_inventory",
                    "workflow",
                    "wasm",
                ]),
            )),
            &mut sql,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }
    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "Published artifacts require forward migrations".into(),
        ))
    }
}
