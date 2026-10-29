mod guard;
use crate::{
    features::{comb, hiveway, identity, pollen, scout, waggle, wax},
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
        .route("/mcp", post(hiveway::gateway))
        .route("/api/ready", get(ready))
        .route("/v1/workflows", post(comb::compose))
        .route("/v1/activity", get(waggle::activity))
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
        .layer(DefaultBodyLimit::disable())
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
            header::HeaderName::from_static("mcp-protocol-version"),
            header::HeaderName::from_static("mcp-method"),
            header::HeaderName::from_static("mcp-name"),
        ])
        .expose_headers([
            header::HeaderName::from_static("payment-required"),
            header::HeaderName::from_static("payment-response"),
        ]))
}

/// Infrastructure readiness, separate from process liveness.
/// # Errors
/// Returns service-unavailable errors when PostgreSQL or Redis is unavailable.
#[utoipa::path(get,path="/api/ready",responses((status=200,description="Infrastructure connected")))]
pub async fn ready(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> crate::features::common::error::ApiResult<axum::Json<serde_json::Value>> {
    use crate::features::common::error::ApiError;
    use fred::prelude::ClientLike;
    state
        .postgres
        .ping()
        .await
        .map_err(|_| ApiError::unavailable("postgres_unavailable"))?;
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        state.redis.ping::<String>(None),
    )
    .await
    .map_err(|_| ApiError::unavailable("redis_unavailable"))?
    .map_err(|_| ApiError::unavailable("redis_unavailable"))?;
    Ok(axum::Json(
        serde_json::json!({"status":"ok","postgres":"ok","redis":"ok"}),
    ))
}

/// Add admission control after attaching shared state.
pub fn protect(router: Router, state: AppState) -> Router {
    let admission = guard::Admission {
        requests: std::sync::Arc::new(tokio::sync::Semaphore::new(
            state.settings.runtime.http.concurrency,
        )),
        peers: std::sync::Arc::default(),
        state,
    };
    router.layer(axum::middleware::from_fn_with_state(
        admission,
        guard::guard,
    ))
}
