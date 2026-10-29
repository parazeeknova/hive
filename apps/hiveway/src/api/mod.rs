use crate::{
    features::{identity, pollen, scout, waggle, wax},
    state::AppState,
};
use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{HeaderValue, Method, header},
    routing::{get, post},
};
use tower_http::cors::CorsLayer;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/auth/challenges", post(identity::challenge))
        .route("/v1/auth/verify", post(identity::verify))
        .route("/v1/credentials/me", get(identity::me))
        .route("/v1/capabilities", get(scout::discover).post(wax::publish))
        .route("/v1/quotes", post(pollen::quote))
        .route(
            "/v1/invocations/{id}",
            get(pollen::get_receipt).post(pollen::invoke),
        )
        .route("/v1/invocations/{id}/reconcile", post(pollen::reconcile))
        .route(
            "/v1/invocations/{id}/checkout",
            post(pollen::dodo::checkout),
        )
        .route("/v1/webhooks/dodo", post(pollen::dodo::webhook))
        .route("/v1/sales", get(waggle::sales))
        .layer(DefaultBodyLimit::max(131_072))
}
/// # Errors
/// Rejects an origin that cannot be represented as an HTTP header.
pub fn cors(origin: &str) -> anyhow::Result<CorsLayer> {
    Ok(CorsLayer::new()
        .allow_origin(origin.parse::<HeaderValue>()?)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::HeaderName::from_static("idempotency-key"),
            header::HeaderName::from_static("payment-signature"),
        ])
        .expose_headers([
            header::HeaderName::from_static("payment-required"),
            header::HeaderName::from_static("payment-response"),
        ]))
}
