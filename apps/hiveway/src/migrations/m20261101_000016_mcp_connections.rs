use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("CREATE TABLE mcp_connections (id uuid PRIMARY KEY, owner text NOT NULL, name text NOT NULL, tools jsonb NOT NULL, updated_at bigint NOT NULL); CREATE INDEX mcp_connections_owner ON mcp_connections(owner); CREATE TABLE mcp_jobs (id uuid PRIMARY KEY, connection_id uuid NOT NULL REFERENCES mcp_connections(id) ON DELETE CASCADE, request jsonb NOT NULL, result jsonb, lease uuid, expires_at bigint NOT NULL); CREATE INDEX mcp_jobs_pending ON mcp_jobs(connection_id, expires_at) WHERE lease IS NULL; CREATE INDEX mcp_jobs_expiry ON mcp_jobs(expires_at)").await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE mcp_jobs; DROP TABLE mcp_connections")
            .await?;
        Ok(())
    }
}
