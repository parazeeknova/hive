use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                sea_orm::Schema::new(sea_orm::DbBackend::Postgres)
                    .create_table_from_entity(crate::entities::account_profiles::Entity),
            )
            .await
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(crate::entities::account_profiles::Entity)
                    .to_owned(),
            )
            .await
    }
}
