//! PostgreSQL migrations managed by `SeaORM`. Existing SQL migration history is adopted
//! only after its checksum and successful application have been verified.
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement};
use sea_orm_migration::prelude::*;
use sha2::{Digest, Sha384};

struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(Migration {
                version: 202_610_290_001,
                name: "m20261029_000001_transaction_core",
                sql: include_str!("../migrations/202610290001_transaction_core.sql"),
            }),
            Box::new(Migration {
                version: 202_610_290_002,
                name: "m20261029_000002_checkout_and_workflows",
                sql: include_str!("../migrations/202610290002_checkout_and_workflows.sql"),
            }),
            Box::new(Migration {
                version: 202_610_290_003,
                name: "m20261029_000003_execution_activity",
                sql: include_str!("../migrations/202610290003_execution_activity.sql"),
            }),
        ]
    }
}

struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}
impl MigrationName for Migration {
    fn name(&self) -> &str {
        self.name
    }
}
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let history = db
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT to_regclass('_sqlx_migrations') IS NOT NULL AS present",
            ))
            .await?
            .ok_or_else(|| DbErr::Custom("migration history lookup returned no row".into()))?;
        if history.try_get::<bool>("", "present")? {
            let old = db
                .query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    "SELECT success, checksum FROM _sqlx_migrations WHERE version=$1",
                    [self.version.into()],
                ))
                .await?;
            if let Some(old) = old {
                if !old.try_get::<bool>("", "success")?
                    || old.try_get::<Vec<u8>>("", "checksum")?
                        != Sha384::digest(self.sql.as_bytes()).to_vec()
                {
                    return Err(DbErr::Custom(format!(
                        "legacy migration {} failed or its checksum changed",
                        self.version
                    )));
                }
                return Ok(());
            }
        }
        db.execute_unprepared(self.sql).await?;
        Ok(())
    }
    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "Hiveway migrations require an explicit forward migration to preserve payment history"
                .into(),
        ))
    }
}

/// Apply pending migrations atomically using `SeaORM`'s PostgreSQL migration runner.
///
/// # Errors
/// Returns database errors or rejects a mismatched legacy migration checksum.
pub async fn migrate(db: &DatabaseConnection) -> Result<(), DbErr> {
    Migrator::up(db, None).await
}
