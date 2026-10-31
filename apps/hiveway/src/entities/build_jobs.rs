use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "build_jobs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub owner: String,
    pub credential_id: Uuid,
    pub idempotency_key: Uuid,
    pub request_hash: Vec<u8>,
    pub request: Json,
    pub state: String,
    pub model: String,
    pub attempts: i32,
    pub source: Option<String>,
    pub validation: Option<Json>,
    pub artifact_id: Option<Uuid>,
    pub capability_id: Option<Uuid>,
    pub error_code: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
