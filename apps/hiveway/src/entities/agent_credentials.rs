use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "agent_credentials")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub wallet: String,
    pub name: String,
    pub token_hash: Vec<u8>,
    pub scopes: Vec<String>,
    pub per_call_limit: i64,
    pub total_limit: i64,
    pub spent: i64,
    pub reserved: i64,
    pub expires_at: i64,
    pub revoked_at: Option<i64>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
