use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("CREATE TABLE mcp_cache (connection_id uuid NOT NULL REFERENCES mcp_connections(id) ON DELETE CASCADE, tool text NOT NULL, args_hash text NOT NULL, result jsonb NOT NULL, expires_at bigint NOT NULL, PRIMARY KEY(connection_id, tool, args_hash)); CREATE INDEX mcp_cache_expiry ON mcp_cache(expires_at)").await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE mcp_cache")
            .await?;
        Ok(())
    }
}
