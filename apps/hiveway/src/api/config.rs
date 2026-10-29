//! Public, explicit integration configuration. Never serialize internal Settings.
use crate::state::AppState;
use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct PublicConfig {
    pub version: &'static str,
    pub public_url: String,
    pub auth_origin: String,
    pub payment: PublicPayment,
    pub features: Features,
    pub limits: PublicLimits,
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
pub struct Features {
    pub dodo_test_checkout: bool,
    pub wasm_artifacts: bool,
}
#[derive(Serialize, ToSchema)]
pub struct PublicLimits {
    pub request_body_bytes: usize,
    pub artifact_bytes: usize,
    pub input_bytes: usize,
    pub output_bytes: usize,
    pub schema_bytes: usize,
}
/// Read public network, origin, feature and size information for clients.
#[utoipa::path(get,path="/api/config",responses((status=200,body=PublicConfig)))]
pub async fn configuration(State(state): State<AppState>) -> Json<PublicConfig> {
    let settings = &state.settings;
    let payment = &settings.runtime.payment;
    let sandbox = &settings.runtime.sandbox;
    Json(PublicConfig {
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
        },
        limits: PublicLimits {
            request_body_bytes: settings.runtime.http.body_bytes,
            artifact_bytes: sandbox.artifact_bytes,
            input_bytes: sandbox.input_bytes,
            output_bytes: sandbox.output_bytes,
            schema_bytes: sandbox.schema_bytes,
        },
    })
}
