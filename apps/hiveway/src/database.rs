//! `SeaORM`'s native migration runner. Schema and application queries use typed builders.
use sea_orm::{DatabaseConnection, DbErr};
use sea_orm_migration::MigratorTrait;
/// Apply native migrations without resetting existing data.
/// # Errors
/// Rejects incompatible migration histories and database failures.
pub async fn migrate(db: &DatabaseConnection) -> Result<(), DbErr> {
    crate::migrations::Migrator::up(db, None).await
}
/// Convert a configured query limit to the type required by `SeaORM`.
/// # Errors
/// Rejects negative limits.
pub fn limit(value: i64) -> Result<u64, DbErr> {
    u64::try_from(value).map_err(|_| DbErr::Type("negative query limit".into()))
}

/// Lock a credential row while completing a financial transaction, including revoked credentials.
/// # Errors
/// Returns database errors or a missing-row error.
pub async fn lock_credential(
    tx: &sea_orm::DatabaseTransaction,
    id: uuid::Uuid,
) -> Result<crate::entities::agent_credentials::Model, DbErr> {
    use sea_orm::{EntityTrait, QuerySelect};
    crate::entities::agent_credentials::Entity::find_by_id(id)
        .lock_exclusive()
        .one(tx)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("credential missing".into()))
}
