use crate::entities::build_jobs::{Column, Entity};
use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                sea_orm::Schema::new(sea_orm::DbBackend::Postgres).create_table_from_entity(Entity),
            )
            .await?;
        for (name, columns, unique) in [
            (
                "build_job_idempotency",
                vec![Column::Owner, Column::IdempotencyKey],
                true,
            ),
            (
                "build_job_queue",
                vec![Column::State, Column::CreatedAt],
                false,
            ),
            (
                "build_job_owner",
                vec![Column::Owner, Column::CreatedAt],
                false,
            ),
        ] {
            let mut index = Index::create();
            index.name(name).table(Entity);
            for col in columns {
                index.col(col);
            }
            if unique {
                index.unique();
            }
            manager.create_index(index.clone()).await?;
        }
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Entity).to_owned())
            .await
    }
}
