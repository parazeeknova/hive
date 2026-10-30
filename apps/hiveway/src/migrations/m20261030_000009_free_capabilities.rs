use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in [
            "ALTER TABLE capabilities DROP CONSTRAINT capabilities_rule_3, ADD CONSTRAINT capabilities_rule_3 CHECK (price BETWEEN 0 AND 1000000000)",
            "ALTER TABLE invocations DROP CONSTRAINT invocations_rule_1, ADD CONSTRAINT invocations_rule_1 CHECK (price >= 0)",
            "ALTER TABLE invocations DROP CONSTRAINT invocations_rule_2, ADD CONSTRAINT invocations_rule_2 CHECK (payment_method IN ('x402', 'dodo') OR (payment_method = 'free' AND price = 0))",
            "ALTER TABLE invocations DROP CONSTRAINT invocations_rule_3, ADD CONSTRAINT invocations_rule_3 CHECK (state <> 'settled' OR (result IS NOT NULL AND (settlement IS NOT NULL OR (price = 0 AND payment_method IS NOT DISTINCT FROM 'free'))))",
        ] {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
    async fn down(&self, _: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "Free invocation history requires a forward migration".into(),
        ))
    }
}
