use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Shared integrations queue jobs from many accounts on one connection;
        // the caller column bounds each account's share of that queue.
        manager
            .get_connection()
            .execute_unprepared("ALTER TABLE mcp_jobs ADD COLUMN IF NOT EXISTS caller text")
            .await?;
        for item in crate::features::integrations::registry() {
            manager.get_connection().execute_unprepared(&format!(
                "INSERT INTO capabilities(id,creator_wallet,name,description,version,runtime,price,active,created_at,definition) VALUES('{}','hive:official','{}','{}','1','connected_mcp',0,true,extract(epoch from now())::bigint,'{}'::jsonb) ON CONFLICT(id) DO NOTHING",
                item.id, item.name.replace('\'', "''"), item.description.replace('\'', "''"),
                serde_json::json!({"tool_name":item.default_tool,"tools":item.tools,"provider":item.slug}).to_string().replace('\'', "''")
            )).await?;
        }
        Ok(())
    }
    async fn down(&self, _: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "Published integration history requires a forward migration".into(),
        ))
    }
}
