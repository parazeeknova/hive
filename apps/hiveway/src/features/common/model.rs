use sea_orm::FromQueryResult;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[cfg(test)]
pub const NETWORK: &str = "eip155:84532";
#[cfg(test)]
pub const ASSET: &str = "0x036CbD53842c5426634e7929541eC2318f3dCF7e";
pub const SCOPES: &[&str] = &[
    "capabilities:read",
    "capabilities:publish",
    "repository:analyze",
    "receipts:read",
    "sales:read",
    "workflows:publish",
];

#[must_use]
pub fn now() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}

#[derive(Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CredentialPolicy {
    pub name: String,
    pub scopes: Vec<String>,
    /// Maximum price of one invocation, in atomic USDC units (6 decimals).
    pub per_call_limit: String,
    /// Cumulative budget for this credential, in atomic USDC units.
    pub total_limit: String,
    pub expires_at: i64,
}

#[derive(Clone, Deserialize, Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum WalletAction {
    Issue { policy: CredentialPolicy },
    Revoke { credential_id: Uuid },
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ChallengeRequest {
    pub wallet: String,
    pub action: WalletAction,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ChallengeResponse {
    pub id: Uuid,
    pub message: String,
    pub expires_at: i64,
}
#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct VerifyRequest {
    pub challenge_id: Uuid,
    pub signature: String,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct AuthResponse {
    pub credential_id: Uuid,
    /// Present only at issuance. Store safely; it cannot be retrieved later.
    pub token: Option<String>,
    pub revoked: bool,
}

#[derive(Clone, FromQueryResult)]
pub struct Credential {
    pub id: Uuid,
    pub wallet: String,
    pub name: String,
    pub scopes: Vec<String>,
    pub per_call_limit: i64,
    pub total_limit: i64,
    pub spent: i64,
    pub reserved: i64,
    pub expires_at: i64,
    pub revoked_at: Option<i64>,
}
#[derive(Serialize, ToSchema)]
pub struct CredentialStatus {
    pub id: Uuid,
    pub wallet: String,
    pub name: String,
    pub scopes: Vec<String>,
    pub per_call_limit: String,
    pub total_limit: String,
    pub spent: String,
    pub reserved: String,
    pub expires_at: i64,
}
impl From<Credential> for CredentialStatus {
    fn from(c: Credential) -> Self {
        Self {
            id: c.id,
            wallet: c.wallet,
            name: c.name,
            scopes: c.scopes,
            per_call_limit: c.per_call_limit.to_string(),
            total_limit: c.total_limit.to_string(),
            spent: c.spent.to_string(),
            reserved: c.reserved.to_string(),
            expires_at: c.expires_at,
        }
    }
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PublishCapability {
    pub name: String,
    pub description: String,
    /// Only `repository_stack` is supported by the initial bounded runtime.
    pub runtime: String,
    pub price: String,
}
#[derive(Serialize, Deserialize, FromQueryResult, ToSchema)]
pub struct Capability {
    pub id: Uuid,
    pub creator_wallet: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub runtime: String,
    #[schema(value_type = String)]
    #[serde(with = "amount_string")]
    pub price: i64,
    pub active: bool,
    pub definition: serde_json::Value,
}
mod amount_string {
    use serde::{Deserialize, Deserializer, Serializer};
    // Serde requires serializers to accept a shared reference to the field.
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn serialize<S: Serializer>(amount: &i64, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&amount.to_string())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<i64, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RepositoryInput {
    pub repository_url: String,
    /// A full immutable Git commit SHA, rather than a moving branch name.
    pub revision: String,
}
#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct QuoteRequest {
    pub capability_id: Uuid,
    pub input: RepositoryInput,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PaymentRequirements {
    pub scheme: String,
    pub network: String,
    pub amount: String,
    pub asset: String,
    pub pay_to: String,
    pub max_timeout_seconds: u32,
    pub extra: serde_json::Value,
}
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Resource {
    pub url: String,
    pub description: String,
    pub mime_type: String,
}
#[derive(Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRequired {
    pub x402_version: u8,
    pub resource: Resource,
    pub accepts: Vec<PaymentRequirements>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentPayload {
    pub x402_version: u8,
    pub resource: Resource,
    pub accepted: PaymentRequirements,
    pub payload: EvmPayload,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct EvmPayload {
    pub signature: String,
    pub authorization: Authorization,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Authorization {
    pub from: String,
    pub to: String,
    pub value: String,
    pub valid_after: String,
    pub valid_before: String,
    pub nonce: String,
}

#[derive(Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Settlement {
    pub success: bool,
    pub transaction: String,
    pub network: String,
    pub payer: Option<String>,
    pub error_reason: Option<String>,
}

#[derive(Clone, FromQueryResult)]
pub struct Invocation {
    pub id: Uuid,
    pub credential_id: Uuid,
    pub capability_id: Uuid,
    pub input: serde_json::Value,
    pub requirements: serde_json::Value,
    pub resource: serde_json::Value,
    pub state: String,
    pub price: i64,
    pub expires_at: i64,
    pub payment_nonce: Option<String>,
    pub payment_payload: Option<serde_json::Value>,
    pub result: Option<serde_json::Value>,
    pub settlement: Option<serde_json::Value>,
    pub settlement_block: Option<String>,
    pub error_code: Option<String>,
    pub payment_method: Option<String>,
    pub checkout_id: Option<String>,
    pub checkout_url: Option<String>,
    pub provider_payment_id: Option<String>,
    pub worker_started_at: Option<i64>,
    pub execution_ms: Option<i64>,
    pub result_bytes: Option<i64>,
    pub payment_issue: Option<String>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct QuoteResponse {
    pub id: Uuid,
    pub expires_at: i64,
    pub payment: PaymentRequired,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct Receipt {
    pub id: Uuid,
    pub capability_id: Uuid,
    pub state: String,
    pub price: String,
    pub result: Option<serde_json::Value>,
    pub settlement: Option<Settlement>,
    pub error_code: Option<String>,
    pub execution_ms: Option<i64>,
    pub result_bytes: Option<i64>,
    pub payment_issue: Option<String>,
}
#[derive(Serialize, FromQueryResult, ToSchema)]
pub struct Sale {
    pub invocation_id: Uuid,
    pub creator_wallet: String,
    pub payer_wallet: String,
    #[schema(value_type = String)]
    #[serde(with = "amount_string")]
    pub amount: i64,
    pub network: String,
    pub transaction_hash: String,
    pub created_at: i64,
}

impl From<crate::entities::capabilities::Model> for Capability {
    fn from(model: crate::entities::capabilities::Model) -> Self {
        Self {
            id: model.id,
            creator_wallet: model.creator_wallet,
            name: model.name,
            description: model.description,
            version: model.version,
            runtime: model.runtime,
            price: model.price,
            active: model.active,
            definition: model.definition,
        }
    }
}
