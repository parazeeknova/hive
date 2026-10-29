use crate::entities::agent_credentials::Column as AgentCredentialsColumn;
use crate::entities::agent_credentials::Entity as AgentCredentialsEntity;
use crate::entities::capabilities::Column as CapabilitiesColumn;
use crate::entities::capabilities::Entity as CapabilitiesEntity;
use crate::entities::creator_sales::Column as CreatorSalesColumn;
use crate::entities::creator_sales::Entity as CreatorSalesEntity;
use crate::entities::invocations::Column as InvocationsColumn;
use crate::entities::invocations::Entity as InvocationsEntity;
use crate::entities::payment_webhook_inbox::Entity as PaymentWebhookInboxEntity;
use crate::entities::wallet_challenges::Column as WalletChallengesColumn;
use crate::entities::wallet_challenges::Entity as WalletChallengesEntity;
use sea_orm::sea_query::{Alias, Expr, ExprTrait, Func};
use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            wallet_challenges(),
            agent_credentials(),
            capabilities(),
            invocations(),
            creator_sales(),
            payment_webhook_inbox(),
        ] {
            manager.create_table(table).await?;
        }
        for index in indexes() {
            manager.create_index(index).await?;
        }
        Ok(())
    }
    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "Payment history requires explicit forward migrations".into(),
        ))
    }
}
fn wallet_challenges() -> TableCreateStatement {
    sea_orm::Schema::new(sea_orm::DbBackend::Postgres)
        .create_table_from_entity(WalletChallengesEntity)
}
fn agent_credentials() -> TableCreateStatement {
    let mut table = sea_orm::Schema::new(sea_orm::DbBackend::Postgres)
        .create_table_from_entity(AgentCredentialsEntity);
    table.check((
        "agent_credentials_rule_0",
        Expr::expr(
            Func::cust(Alias::new("octet_length"))
                .args([Expr::col(AgentCredentialsColumn::TokenHash)]),
        )
        .eq(Expr::val(32_i64)),
    ));
    table.check((
        "agent_credentials_rule_1",
        Expr::col(AgentCredentialsColumn::PerCallLimit).gt(Expr::val(0_i64)),
    ));
    table.check((
        "agent_credentials_rule_2",
        Expr::col(AgentCredentialsColumn::TotalLimit).gt(Expr::val(0_i64)),
    ));
    table.check((
        "agent_credentials_rule_3",
        Expr::col(AgentCredentialsColumn::Spent).gte(Expr::val(0_i64)),
    ));
    table.check((
        "agent_credentials_rule_4",
        Expr::col(AgentCredentialsColumn::Reserved).gte(Expr::val(0_i64)),
    ));
    table.check((
        "agent_credentials_rule_5",
        Expr::col(AgentCredentialsColumn::Spent)
            .add(Expr::col(AgentCredentialsColumn::Reserved))
            .lte(Expr::col(AgentCredentialsColumn::TotalLimit)),
    ));
    table
}
fn capabilities() -> TableCreateStatement {
    let mut table = sea_orm::Schema::new(sea_orm::DbBackend::Postgres)
        .create_table_from_entity(CapabilitiesEntity);
    table.check((
        "capabilities_rule_0",
        Expr::expr(Func::char_length(Expr::col(CapabilitiesColumn::Name)))
            .between(Expr::val(1_i64), Expr::val(80_i64)),
    ));
    table.check((
        "capabilities_rule_1",
        Expr::expr(Func::char_length(Expr::col(
            CapabilitiesColumn::Description,
        )))
        .between(Expr::val(1_i64), Expr::val(1000_i64)),
    ));
    table.check((
        "capabilities_rule_2",
        Expr::col(CapabilitiesColumn::Runtime).is_in([
            "repository_stack",
            "dependency_inventory",
            "workflow",
            "wasm",
        ]),
    ));
    table.check((
        "capabilities_rule_3",
        Expr::col(CapabilitiesColumn::Price)
            .between(Expr::val(1_i64), Expr::val(1_000_000_000_i64)),
    ));
    table
}
fn invocations() -> TableCreateStatement {
    let mut table = sea_orm::Schema::new(sea_orm::DbBackend::Postgres)
        .create_table_from_entity(InvocationsEntity);
    table.check(("invocations_rule_0", invocation_states()));
    table.check((
        "invocations_rule_1",
        Expr::col(InvocationsColumn::Price).gt(Expr::val(0_i64)),
    ));
    table.check((
        "invocations_rule_2",
        Expr::col(InvocationsColumn::PaymentMethod).is_in(["x402", "dodo"]),
    ));
    table.check(("invocations_rule_3", settled_output()));
    table
}
fn creator_sales() -> TableCreateStatement {
    let mut table = sea_orm::Schema::new(sea_orm::DbBackend::Postgres)
        .create_table_from_entity(CreatorSalesEntity);
    table.check((
        "creator_sales_rule_0",
        Expr::col(CreatorSalesColumn::Amount).gt(Expr::val(0_i64)),
    ));
    table
}
fn payment_webhook_inbox() -> TableCreateStatement {
    sea_orm::Schema::new(sea_orm::DbBackend::Postgres)
        .create_table_from_entity(PaymentWebhookInboxEntity)
}
fn indexes() -> Vec<IndexCreateStatement> {
    vec![
        Index::create()
            .name("wallet_challenges_ip_rate")
            .table(WalletChallengesEntity)
            .col(WalletChallengesColumn::PeerIp)
            .col(WalletChallengesColumn::CreatedAt)
            .to_owned(),
        Index::create()
            .name("wallet_challenges_rate")
            .table(WalletChallengesEntity)
            .col(WalletChallengesColumn::Wallet)
            .col(WalletChallengesColumn::CreatedAt)
            .to_owned(),
        Index::create()
            .name("agent_credentials_owner")
            .table(AgentCredentialsEntity)
            .col(AgentCredentialsColumn::Wallet)
            .to_owned(),
        Index::create()
            .name("agent_credentials_token_hash_key")
            .table(AgentCredentialsEntity)
            .unique()
            .col(AgentCredentialsColumn::TokenHash)
            .to_owned(),
        Index::create()
            .name("invocations_credential_id_idempotency_key_key")
            .table(InvocationsEntity)
            .unique()
            .col(InvocationsColumn::CredentialId)
            .col(InvocationsColumn::IdempotencyKey)
            .to_owned(),
        Index::create()
            .name("invocations_payment_nonce_key")
            .table(InvocationsEntity)
            .unique()
            .col(InvocationsColumn::PaymentNonce)
            .to_owned(),
        Index::create()
            .name("invocations_checkout_id_key")
            .table(InvocationsEntity)
            .unique()
            .col(InvocationsColumn::CheckoutId)
            .to_owned(),
        Index::create()
            .name("invocations_provider_payment_id_key")
            .table(InvocationsEntity)
            .unique()
            .col(InvocationsColumn::ProviderPaymentId)
            .to_owned(),
        Index::create()
            .name("invocations_expiry")
            .table(InvocationsEntity)
            .col(InvocationsColumn::CredentialId)
            .col(InvocationsColumn::ExpiresAt)
            .and_where(Expr::col(InvocationsColumn::State).eq(Expr::val("quoted")))
            .to_owned(),
        Index::create()
            .name("invocations_recovery")
            .table(InvocationsEntity)
            .col(InvocationsColumn::State)
            .and_where(Expr::col(InvocationsColumn::State).is_in([
                "verifying",
                "executing",
                "settling",
                "settlement_unknown",
            ]))
            .to_owned(),
        Index::create()
            .name("invocations_worker")
            .table(InvocationsEntity)
            .col(InvocationsColumn::State)
            .col(InvocationsColumn::UpdatedAt)
            .to_owned(),
        Index::create()
            .name("invocations_owner_activity")
            .table(InvocationsEntity)
            .col(InvocationsColumn::CredentialId)
            .col(InvocationsColumn::CreatedAt)
            .to_owned(),
        Index::create()
            .name("creator_sales_transaction_hash_key")
            .table(CreatorSalesEntity)
            .unique()
            .col(CreatorSalesColumn::TransactionHash)
            .to_owned(),
    ]
}

pub(super) fn invocation_states() -> Expr {
    Expr::col(InvocationsColumn::State).is_in([
        "quoted",
        "verifying",
        "executing",
        "settling",
        "settlement_unknown",
        "settled",
        "failed",
        "expired",
        "checkout_creating",
        "checkout_pending",
        "checkout_review",
        "paid",
        "execution_failed",
    ])
}
pub(super) fn settled_output() -> Expr {
    Expr::col(InvocationsColumn::State)
        .ne("settled")
        .or(Expr::col(InvocationsColumn::Result)
            .is_not_null()
            .and(Expr::col(InvocationsColumn::Settlement).is_not_null()))
}
