//! Public, explicit integration configuration. Never serialize internal Settings.
use crate::state::AppState;
use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct PublicConfig {
    pub builder: Option<PublicBuilder>,
    pub version: &'static str,
    pub public_url: String,
    pub auth_origin: String,
    pub payment: PublicPayment,
    pub features: Features,
    pub limits: PublicLimits,
    pub connectors: ConnectorConfig,
    pub integrations: Vec<PublicIntegration>,
}
#[derive(Serialize, ToSchema)]
pub struct PublicBuilder {
    pub model: String,
    pub account_daily_limit: u64,
    pub global_daily_limit: u64,
}
#[derive(Serialize, ToSchema)]
pub struct PublicPayment {
    pub network: String,
    pub chain_id: u64,
    pub asset: String,
    pub decimals: u8,
    pub name: String,
    pub version: String,
    pub settlement_finality: &'static str,
    pub creator_payouts: &'static str,
}
#[derive(Serialize, ToSchema)]
#[allow(clippy::struct_excessive_bools)]
pub struct Features {
    pub dodo_test_checkout: bool,
    pub wasm_artifacts: bool,
    pub mcp_oauth: bool,
    pub semantic_tool_search: bool,
    pub code_index: bool,
    pub repository_memory: bool,
    /// Accounts can save provider API keys in the dashboard.
    pub provider_keys: bool,
}
/// Limits connectors, Hive Local and the code-index CLI read instead of hardcoding.
#[derive(Serialize, ToSchema)]
pub struct ConnectorConfig {
    pub argument_bytes: usize,
    pub result_bytes: usize,
    pub poll_wait_seconds: u64,
    pub job_ttl_seconds: i64,
    pub input_requests_per_job: usize,
    pub code_file_bytes: usize,
    pub code_manifest_files_per_call: usize,
    pub code_blobs_per_upload: usize,
}
/// A bring-your-own-key provider Hive Local can connect to on the user's machine.
#[derive(Serialize, ToSchema)]
pub struct PublicIntegration {
    pub slug: String,
    pub name: String,
    pub byok: bool,
    pub upstream: Option<crate::features::integrations::Upstream>,
}
#[derive(Serialize, ToSchema)]
pub struct PublicLimits {
    pub request_body_bytes: usize,
    pub artifact_bytes: usize,
    pub input_bytes: usize,
    pub output_bytes: usize,
    pub schema_bytes: usize,
    pub json_depth: usize,
    pub json_nodes: usize,
    pub collection_items: usize,
}
/// Read public network, origin, feature and size information for clients.
#[utoipa::path(get,path="/api/config",responses((status=200,body=PublicConfig)))]
pub async fn configuration(State(state): State<AppState>) -> Json<PublicConfig> {
    let settings = &state.settings;
    let payment = &settings.runtime.payment;
    let sandbox = &settings.runtime.sandbox;
    Json(PublicConfig {
        builder: settings.builder.as_ref().map(|config| PublicBuilder {
            model: config.model.clone(),
            account_daily_limit: config.owner_daily,
            global_daily_limit: config.global_daily,
        }),
        version: env!("CARGO_PKG_VERSION"),
        public_url: settings.public_url.clone(),
        auth_origin: settings.auth_uri.clone(),
        payment: PublicPayment {
            network: payment.network(),
            chain_id: payment.chain_id,
            asset: payment.asset.clone(),
            decimals: payment.decimals,
            name: payment.name.clone(),
            version: payment.version.clone(),
            settlement_finality: payment.settlement_finality.tag(),
            creator_payouts: "direct_x402",
        },
        features: Features {
            dodo_test_checkout: settings.dodo_api_key.is_some()
                && settings.dodo_product_id.is_some()
                && settings.dodo_webhook_secret.is_some(),
            wasm_artifacts: true,
            mcp_oauth: true,
            semantic_tool_search: settings.embeddings.is_some(),
            code_index: true,
            repository_memory: true,
            provider_keys: settings.secrets_key.is_some(),
        },
        connectors: ConnectorConfig {
            argument_bytes: settings.runtime.connectors.argument_bytes,
            result_bytes: settings.runtime.connectors.result_bytes,
            poll_wait_seconds: settings.runtime.connectors.poll_wait_seconds,
            job_ttl_seconds: settings.runtime.connectors.job_ttl_seconds,
            input_requests_per_job: settings.runtime.connectors.input_requests_per_job,
            code_file_bytes: settings.runtime.code_index.file_bytes,
            code_manifest_files_per_call: settings.runtime.code_index.manifest_files_per_call,
            code_blobs_per_upload: settings.runtime.code_index.blobs_per_upload,
        },
        integrations: crate::features::integrations::registry()
            .iter()
            .map(|item| PublicIntegration {
                slug: item.slug.clone(),
                name: item.name.clone(),
                byok: item.byok,
                upstream: item.upstream.clone(),
            })
            .collect(),
        limits: PublicLimits {
            request_body_bytes: settings.runtime.http.body_bytes,
            artifact_bytes: sandbox.artifact_bytes,
            input_bytes: sandbox.input_bytes,
            output_bytes: sandbox.output_bytes,
            schema_bytes: sandbox.schema_bytes,
            json_depth: sandbox.json_depth,
            json_nodes: sandbox.json_nodes,
            collection_items: sandbox.collection_items,
        },
    })
}
