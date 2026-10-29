use crate::entities::capabilities::{Column, Entity};
use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for creator in [false, true] {
            let mut index = Index::create();
            index
                .name(if creator {
                    "capability_creator_catalog"
                } else {
                    "capability_active_catalog"
                })
                .table(Entity);
            if creator {
                index.col(Column::CreatorWallet);
            }
            index
                .col(Column::Active)
                .col((Column::CreatedAt, IndexOrder::Desc))
                .col((Column::Id, IndexOrder::Asc));
            manager.create_index(index.clone()).await?;
        }
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for name in ["capability_creator_catalog", "capability_active_catalog"] {
            manager
                .drop_index(Index::drop().name(name).to_owned())
                .await?;
        }
        Ok(())
    }
}
