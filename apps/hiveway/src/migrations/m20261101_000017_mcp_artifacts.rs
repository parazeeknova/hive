use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection().execute_unprepared("ALTER TABLE capabilities DROP CONSTRAINT capabilities_rule_2; ALTER TABLE capabilities ADD CONSTRAINT capabilities_rule_2 CHECK(runtime IN ('repository_stack','dependency_inventory','workflow','wasm','repository_scaffold','connected_mcp')); CREATE TABLE mcp_artifact_attempts(invocation_id uuid NOT NULL REFERENCES invocations(id) ON DELETE CASCADE, step integer NOT NULL, PRIMARY KEY(invocation_id,step))").await?;
        Ok(())
    }
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection().execute_unprepared("DROP TABLE mcp_artifact_attempts; ALTER TABLE capabilities DROP CONSTRAINT capabilities_rule_2; ALTER TABLE capabilities ADD CONSTRAINT capabilities_rule_2 CHECK(runtime IN ('repository_stack','dependency_inventory','workflow','wasm','repository_scaffold'))").await?;
        Ok(())
    }
}
