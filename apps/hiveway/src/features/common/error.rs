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
}

#[derive(Serialize, ToSchema)]
pub struct ErrorBody {
    pub error: String,
}

impl ApiError {
    #[must_use]
    pub const fn new(status: StatusCode, code: &'static str) -> Self {
        Self { status, code }
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
