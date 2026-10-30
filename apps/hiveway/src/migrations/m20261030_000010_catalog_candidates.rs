use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in [
            "CREATE INDEX capability_lower_name_trgm ON capabilities USING gin (lower(name) public.gin_trgm_ops)",
            "CREATE INDEX capability_lower_description_trgm ON capabilities USING gin (lower(description) public.gin_trgm_ops)",
        ] {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in [
            "DROP INDEX capability_lower_name_trgm",
            "DROP INDEX capability_lower_description_trgm",
        ] {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
