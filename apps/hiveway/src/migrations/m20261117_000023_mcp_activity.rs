use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "CREATE TABLE mcp_activity (
                id uuid PRIMARY KEY,
                owner text NOT NULL,
                credential_id uuid NOT NULL REFERENCES agent_credentials(id) ON DELETE CASCADE,
                tool text NOT NULL,
                provider text,
                client_context jsonb,
                run_id uuid,
                task_id uuid,
                state text NOT NULL CHECK (state IN ('running','accepted','succeeded','failed','interrupted')),
                error_code text,
                execution_ms bigint,
                created_at bigint NOT NULL,
                completed_at bigint,
                deadline bigint NOT NULL,
                expires_at bigint NOT NULL
            );
            CREATE INDEX mcp_activity_owner ON mcp_activity(owner, created_at DESC, id DESC);
            CREATE INDEX mcp_activity_credential ON mcp_activity(credential_id, created_at DESC, id DESC);
            CREATE INDEX mcp_activity_pending ON mcp_activity(deadline) WHERE state IN ('running','accepted');
            CREATE INDEX mcp_activity_expiry ON mcp_activity(expires_at);"
        ).await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE mcp_activity")
            .await?;
        Ok(())
    }
}
