use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "capabilities")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub creator_wallet: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub runtime: String,
    pub price: i64,
    #[sea_orm(default_value = true)]
    pub active: bool,
    pub created_at: i64,
    #[sea_orm(default_value = "{}")]
    pub definition: Json,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
