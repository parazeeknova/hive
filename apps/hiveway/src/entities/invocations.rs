use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "invocations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub credential_id: Uuid,
    pub capability_id: Uuid,
    pub idempotency_key: Uuid,
    pub request_hash: Vec<u8>,
    pub input: Json,
    pub requirements: Json,
    pub resource: Json,
    pub state: String,
    pub price: i64,
    pub expires_at: i64,
    pub payment_nonce: Option<String>,
    pub payment_payload: Option<Json>,
    pub result: Option<Json>,
    pub settlement: Option<Json>,
    pub settlement_block: Option<String>,
    pub error_code: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub payment_method: Option<String>,
    pub checkout_id: Option<String>,
    pub checkout_url: Option<String>,
    pub provider_payment_id: Option<String>,
    pub creator_wallet: String,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub execution_ms: Option<i64>,
    pub result_bytes: Option<i64>,
    pub payment_issue: Option<String>,
    pub worker_started_at: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::agent_credentials::Entity",
        from = "Column::CredentialId",
        to = "super::agent_credentials::Column::Id"
    )]
    Credential,
    #[sea_orm(
        belongs_to = "super::capabilities::Entity",
        from = "Column::CapabilityId",
        to = "super::capabilities::Column::Id"
    )]
    Capability,
}
impl Related<super::agent_credentials::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Credential.def()
    }
}
impl Related<super::capabilities::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Capability.def()
    }
}
impl ActiveModelBehavior for ActiveModel {}
