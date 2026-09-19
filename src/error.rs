/// Unified application error type.
///
/// Every error the API can produce is a variant of this enum, ensuring all
/// error responses follow the structured JSON format specified in FR09:
/// `{"error": "message", "code": "ERROR_CODE"}`.
///
/// No stack traces or internal `Debug` output are ever placed in the response
/// body (NFR §8.4).
#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("database error")]
    Database(#[from] sqlx::Error),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("not found")]
    NotFound,

    #[error("internal error")]
    Internal(#[from] anyhow::Error),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("bench already running")]
    BenchAlreadyRunning,
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, code) = match &self {
            AppError::Database(_) => (axum::http::StatusCode::SERVICE_UNAVAILABLE, "DB_ERROR"),
            AppError::Validation(_) => (
                axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                "VALIDATION_ERROR",
            ),
            AppError::NotFound => (axum::http::StatusCode::NOT_FOUND, "NOT_FOUND"),
            AppError::Internal(_) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
            ),
            AppError::Conflict(_) => (axum::http::StatusCode::CONFLICT, "CONFLICT"),
            AppError::BenchAlreadyRunning => (axum::http::StatusCode::CONFLICT, "BENCH_RUNNING"),
        };

        let body = serde_json::json!({
            "error": self.to_string(),
            "code": code,
        });

        (status, axum::Json(body)).into_response()
    }
}
