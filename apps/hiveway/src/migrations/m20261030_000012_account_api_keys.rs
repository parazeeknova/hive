use crate::entities::{
    account_api_keys::{Column, Entity},
    agent_credentials::{Column as CredentialColumn, Entity as Credentials},
};
use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut table =
            sea_orm::Schema::new(sea_orm::DbBackend::Postgres).create_table_from_entity(Entity);
        table.foreign_key(
            ForeignKey::create()
                .from(Entity, Column::Id)
                .to(Credentials, CredentialColumn::Id)
                .on_delete(ForeignKeyAction::Cascade),
        );
        manager.create_table(table).await?;
        manager
            .create_index(
                Index::create()
                    .name("account_api_key_owner")
                    .table(Entity)
                    .col(Column::Owner)
                    .to_owned(),
            )
            .await?;
        // Adopt credentials issued by the previous free-agent endpoint, without tagging browser sessions.
        manager.get_connection().execute_unprepared("INSERT INTO account_api_keys (id, owner) SELECT id, wallet FROM agent_credentials WHERE name = 'Hive free agent' AND wallet LIKE 'github:%'").await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Entity).to_owned())
            .await
    }
}
