use super::m20261029_000001_create_hiveway::{invocation_states, settled_output};
use crate::entities::invocations::Entity;
use sea_orm::sea_query::{Check, IntoIden, PostgresQueryBuilder, TableBuilder};
use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for (name, expression) in [
            ("invocations_rule_0", invocation_states()),
            ("invocations_rule_3", settled_output()),
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(Entity)
                        .drop_constraint(name)
                        .to_owned(),
                )
                .await?;
            // SeaQuery lacks ALTER ADD CHECK. Render identifiers and the entire
            // predicate with its PostgreSQL builder; only the DDL scaffolding is literal.
            let mut sql = String::from("ALTER TABLE ");
            PostgresQueryBuilder.prepare_iden(&Entity.into_iden(), &mut sql);
            sql.push_str(" ADD ");
            PostgresQueryBuilder
                .prepare_check_constraint(&Check::from((name, expression)), &mut sql);
            manager.get_connection().execute_unprepared(&sql).await?;
        }
        Ok(())
    }
    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "Payment constraints require forward migrations".into(),
        ))
    }
}
