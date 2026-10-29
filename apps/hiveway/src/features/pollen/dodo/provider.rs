use crate::{
    features::{
        common::error::{ApiError, ApiResult},
        pollen::x402,
    },
    state::AppState,
};
use serde_json::Value;
pub(super) async fn request(
    state: &AppState,
    method: reqwest::Method,
    path: &str,
    body: Option<Value>,
) -> ApiResult<Value> {
    let key = state
        .settings
        .dodo_api_key
        .as_ref()
        .ok_or_else(|| ApiError::unavailable("dodo_not_configured"))?;
    let mut request = state
        .http
        .request(method, format!("{}{path}", state.settings.dodo_api_url))
        .bearer_auth(key);
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request
        .send()
        .await
        .map_err(|_| ApiError::unavailable("dodo_unavailable"))?;
    if matches!(response.status().as_u16(), 401 | 403) {
        return Err(ApiError::new(
            axum::http::StatusCode::UNAUTHORIZED,
            "dodo_unauthorized",
        ));
    }
    if definitive_rejection(response.status()) {
        return Err(ApiError::bad("dodo_request_rejected"));
    }
    x402::json_response(response).await
}
pub(super) fn definitive_rejection(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 400 | 404 | 422)
}
pub(super) fn identifier(value: &str) -> ApiResult<&str> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err(ApiError::bad("invalid_provider_id"));
    }
    Ok(value)
}
