use crate::entities::creator_sales::{Column, Entity};
use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .name("creator_sales_creator_activity")
                    .table(Entity)
                    .col(Column::CreatorWallet)
                    .col((Column::CreatedAt, IndexOrder::Desc))
                    .col((Column::InvocationId, IndexOrder::Desc))
                    .to_owned(),
            )
            .await
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("creator_sales_creator_activity")
                    .to_owned(),
            )
            .await
    }
}
