use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "account_profiles")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub owner: String,
    #[sea_orm(unique)]
    pub username: String,
    pub name: String,
    #[sea_orm(default_value = false)]
    pub publisher_verified: bool,
    pub avatar: String,
    pub updated_at: i64,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
