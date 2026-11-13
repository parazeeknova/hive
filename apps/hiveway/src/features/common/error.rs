use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, thiserror::Error)]
#[error("{code}")]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    /// A human-readable explanation for callers, e.g. which workflow step is invalid.
    pub detail: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ErrorBody {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl ApiError {
    #[must_use]
    pub const fn new(status: StatusCode, code: &'static str) -> Self {
        Self {
            status,
            code,
            detail: None,
        }
    }
    /// A 400 error with an explanation of what to change.
    #[must_use]
    pub fn detailed(code: &'static str, detail: impl Into<String>) -> Self {
        let detail: String = detail.into().chars().take(500).collect();
        Self {
            status: StatusCode::BAD_REQUEST,
            code,
            detail: Some(detail),
        }
    }
    #[must_use]
    pub const fn bad(code: &'static str) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code)
    }
    #[must_use]
    pub const fn forbidden() -> Self {
        Self::new(StatusCode::FORBIDDEN, "forbidden")
    }
    #[must_use]
    pub const fn unavailable(code: &'static str) -> Self {
        Self::new(StatusCode::SERVICE_UNAVAILABLE, code)
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorBody {
                error: self.code.to_owned(),
                detail: self.detail,
            }),
        )
            .into_response()
    }
}
impl From<sea_orm::DbErr> for ApiError {
    fn from(error: sea_orm::DbErr) -> Self {
        tracing::error!(%error, "Database operation failed");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "database_error")
    }
}
impl From<serde_json::Error> for ApiError {
    fn from(error: serde_json::Error) -> Self {
        tracing::error!(%error, "Stored data is invalid");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "stored_data_error")
    }
}
pub type ApiResult<T> = Result<T, ApiError>;
