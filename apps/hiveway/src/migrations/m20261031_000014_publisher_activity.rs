use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("ALTER TABLE account_profiles ADD COLUMN IF NOT EXISTS publisher_verified boolean NOT NULL DEFAULT false; ALTER TABLE invocations ADD COLUMN IF NOT EXISTS client_context jsonb; CREATE INDEX invocations_account_activity ON invocations (credential_id, created_at DESC, id DESC); CREATE INDEX invocations_capability_runs ON invocations (capability_id) WHERE started_at IS NOT NULL").await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("DROP INDEX invocations_capability_runs; DROP INDEX invocations_account_activity; ALTER TABLE invocations DROP COLUMN client_context; ALTER TABLE account_profiles DROP COLUMN publisher_verified").await?;
        Ok(())
    }
}
