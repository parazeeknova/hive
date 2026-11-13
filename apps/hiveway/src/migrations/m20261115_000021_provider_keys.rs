//! Provider API keys saved from the dashboard, encrypted with `HIVE_SECRETS_KEY`.
use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r"
CREATE TABLE account_provider_keys (
  owner text NOT NULL,
  slug text NOT NULL,
  ciphertext bytea NOT NULL,
  nonce bytea NOT NULL,
  hint text NOT NULL,
  tools integer NOT NULL DEFAULT 0,
  last_error text,
  created_at bigint NOT NULL,
  updated_at bigint NOT NULL,
  PRIMARY KEY(owner, slug)
);
CREATE INDEX mcp_connections_hosted ON mcp_connections(owner, name) WHERE 'hosted' = ANY(features);
",
            )
            .await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DROP INDEX mcp_connections_hosted; DROP TABLE account_provider_keys",
            )
            .await?;
        Ok(())
    }
}
