use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Multi-tool workflows: official recipes, ones accounts publish, and ones
        // Hive publishes automatically from repeated tool chains. Each step names
        // a Hive tool; runs execute with the caller's own credential.
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE hive_workflows (
                    id uuid PRIMARY KEY,
                    owner text NOT NULL,
                    slug text NOT NULL,
                    name text NOT NULL,
                    description text NOT NULL,
                    definition jsonb NOT NULL,
                    visibility text NOT NULL CHECK (visibility IN ('private','public')),
                    origin text NOT NULL CHECK (origin IN ('official','manual','auto')),
                    signature text,
                    runs bigint NOT NULL DEFAULT 0,
                    failures bigint NOT NULL DEFAULT 0,
                    active boolean NOT NULL DEFAULT true,
                    created_at bigint NOT NULL,
                    updated_at bigint NOT NULL,
                    UNIQUE (owner, slug)
                );
                CREATE INDEX hive_workflows_public ON hive_workflows (visibility, active) WHERE active;
                CREATE UNIQUE INDEX hive_workflows_signature ON hive_workflows (owner, signature) WHERE signature IS NOT NULL;
                CREATE TABLE workflow_runs (
                    id uuid PRIMARY KEY,
                    owner text NOT NULL,
                    workflow_id uuid,
                    definition jsonb NOT NULL,
                    inputs jsonb NOT NULL,
                    status text NOT NULL CHECK (status IN ('running','completed','failed','interrupted')),
                    steps jsonb NOT NULL DEFAULT '{}'::jsonb,
                    output jsonb,
                    error text,
                    tool_calls integer NOT NULL DEFAULT 0,
                    created_at bigint NOT NULL,
                    updated_at bigint NOT NULL,
                    expires_at bigint NOT NULL
                );
                CREATE INDEX workflow_runs_owner ON workflow_runs (owner, status);
                CREATE INDEX workflow_runs_expiry ON workflow_runs (expires_at);
                CREATE TABLE workflow_patterns (
                    owner text NOT NULL,
                    signature text NOT NULL,
                    template jsonb NOT NULL,
                    literals jsonb NOT NULL,
                    varying jsonb NOT NULL DEFAULT '[]'::jsonb,
                    seen integer NOT NULL DEFAULT 0,
                    last_seen bigint NOT NULL,
                    workflow_id uuid,
                    PRIMARY KEY (owner, signature)
                );
                CREATE INDEX workflow_patterns_expiry ON workflow_patterns (last_seen) WHERE workflow_id IS NULL;",
            )
            .await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DROP TABLE workflow_patterns; DROP TABLE workflow_runs; DROP TABLE hive_workflows;",
            )
            .await?;
        Ok(())
    }
}
