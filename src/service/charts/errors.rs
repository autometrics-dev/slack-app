use crate::db::DbError;
use axum::extract::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
#[serde(tag = "error", content = "details", rename_all = "snake_case")]
pub enum ChartHandlerError {
    #[error("Database error: {0}")]
    DatabaseError(DbError),

    #[error("Cannot read file: {0}")]
    FileError(String),

    #[error("Entity not found")]
    NotFound,
}

impl From<DbError> for ChartHandlerError {
    fn from(error: DbError) -> Self {
        match error {
            DbError::NotFound => ChartHandlerError::NotFound,
            error => ChartHandlerError::DatabaseError(error),
        }
    }
}

impl From<std::io::Error> for ChartHandlerError {
    fn from(error: std::io::Error) -> Self {
        Self::FileError(error.to_string())
    }
}

impl IntoResponse for ChartHandlerError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match self {
            Self::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::FileError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::NotFound => StatusCode::NOT_FOUND,
        };

        (status_code, Json(self)).into_response()
    }
}

#[derive(Debug, Error, Serialize)]
#[serde(tag = "error", content = "details", rename_all = "snake_case")]
pub enum ChartServiceError {
    #[error("Cannot generate chart")]
    Generation,

    #[error("Cannot render chart: {0}")]
    Render(String),

    #[error("Cannot store chart found")]
    Storage(String),
}

impl From<std::io::Error> for ChartServiceError {
    fn from(error: std::io::Error) -> Self {
        Self::Storage(error.to_string())
    }
}
