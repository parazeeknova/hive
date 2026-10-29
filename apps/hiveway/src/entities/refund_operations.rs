use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "refund_operations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub invocation_id: Uuid,
    #[sea_orm(unique)]
    pub payment_id: String,
    #[sea_orm(unique)]
    pub refund_id: Option<String>,
    pub state: String,
    pub amount: i64,
    pub error_code: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::invocations::Entity",
        from = "Column::InvocationId",
        to = "super::invocations::Column::Id"
    )]
    Invocation,
}
impl Related<super::invocations::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Invocation.def()
    }
}
impl ActiveModelBehavior for ActiveModel {}
