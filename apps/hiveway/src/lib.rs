pub mod api;
pub mod config;
pub mod database;
pub mod entities;
pub mod features;
pub mod state;
pub mod telemetry;

use std::collections::HashMap;

use axum::{
    Json, Router,
    extract::Request,
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use opentelemetry::global;
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use utoipa::{OpenApi, ToSchema};

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Health {
    pub status: String,
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Hiveway",
        description = "Hiveway gateway: wallet identities, scoped credentials, paid capabilities and receipts"
    ),
    paths(
        health,
        api::ready,
        features::comb::compose,
        features::waggle::activity,
        features::identity::challenge,
        features::identity::verify,
        features::identity::me,
        features::scout::discover,
        features::wax::publish,
        features::pollen::quote,
        features::pollen::invoke,
        features::pollen::get_receipt,
        features::pollen::reconcile,
        features::pollen::dodo::checkout,
        features::pollen::dodo::webhook,
        features::waggle::sales
    ),
    components(schemas(Health))
)]
struct ApiDoc;

/// Build Hiveway's infrastructure routes without starting application workflows.
pub fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/openapi.json", get(openapi))
}

/// Apply request tracing to every API route, including paid invocations.
pub fn observe<S: Clone + Send + Sync + 'static>(router: Router<S>) -> Router<S> {
    router.layer(
        ServiceBuilder::new()
            .layer(middleware::from_fn(request_context))
            .layer(TraceLayer::new_for_http()),
    )
}

#[utoipa::path(get, path = "/api/health", responses((status = 200, body = Health)))]
async fn health() -> Json<Health> {
    tracing::info!(service = "hiveway", "Health request");
    Json(Health {
        status: "ok".to_owned(),
    })
}

async fn openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

async fn request_context(request: Request, next: Next) -> Response {
    let headers: HashMap<String, String> = ["traceparent", "tracestate"]
        .into_iter()
        .filter_map(|name| {
            request
                .headers()
                .get(name)?
                .to_str()
                .ok()
                .map(|value| (name.to_owned(), value.to_owned()))
        })
        .collect();
    let parent = global::get_text_map_propagator(|propagator| propagator.extract(&headers));
    let span = tracing::info_span!(
        "http.request",
        otel.kind = "server",
        http.request.method = %request.method(),
        url.path = %request.uri().path(),
        http.response.status_code = tracing::field::Empty,
    );
    let _ = span.set_parent(parent);
    async move {
        let response = next.run(request).await;
        tracing::Span::current().record("http.response.status_code", response.status().as_u16());
        response
    }
    .instrument(span)
    .await
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test(flavor = "current_thread")]
    async fn health_returns_ok() -> Result<(), Box<dyn std::error::Error>> {
        let request = Request::builder().uri("/api/health").body(Body::empty())?;
        let response = super::router::<()>().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);
        let health: super::Health =
            serde_json::from_slice(&to_bytes(response.into_body(), 1024).await?)?;
        assert_eq!(health.status, "ok");
        Ok(())
    }
}
