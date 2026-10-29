use crate::entities::wasm_artifacts;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UploadArtifact {
    pub wasm_base64: String,
    pub input_schema: Value,
    pub output_schema: Value,
}
#[derive(Serialize, ToSchema)]
pub struct Artifact {
    pub id: Uuid,
    pub owner_wallet: String,
    /// SHA-256 over the code and its canonical input/output schemas.
    pub sha256: String,
    pub byte_length: usize,
    pub abi_version: u8,
    /// ABI and sandbox validation, not a guarantee of business correctness.
    pub verification: &'static str,
    pub input_schema: Value,
    pub output_schema: Value,
    pub created_at: i64,
}
impl From<wasm_artifacts::Model> for Artifact {
    fn from(value: wasm_artifacts::Model) -> Self {
        Self {
            id: value.id,
            owner_wallet: value.owner_wallet,
            sha256: hex::encode(value.digest),
            byte_length: value.wasm.len(),
            abi_version: 1,
            verification: "sandbox_validated",
            input_schema: value.input_schema,
            output_schema: value.output_schema,
            created_at: value.created_at,
        }
    }
}
