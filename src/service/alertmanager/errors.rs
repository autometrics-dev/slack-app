use crate::db::DbError;
use crate::events::Event;
use axum::extract::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

#[derive(Debug, Error, Serialize)]
#[serde(tag = "error", content = "details", rename_all = "snake_case")]
pub enum AlertmanagerWebhookHandlerError {
    #[error("Cannot send message to channel")]
    ChannelClosed,

    #[error("Database error: {0}")]
    DatabaseError(DbError),

    #[error("Entity not found")]
    NotFound,

    #[error("Unexpected version")]
    UnexpectedVersion(String),
}

impl From<DbError> for AlertmanagerWebhookHandlerError {
    fn from(error: DbError) -> Self {
        match error {
            DbError::NotFound => AlertmanagerWebhookHandlerError::NotFound,
            error => AlertmanagerWebhookHandlerError::DatabaseError(error),
        }
    }
}

impl From<SendError<Event>> for AlertmanagerWebhookHandlerError {
    fn from(_error: SendError<Event>) -> Self {
        Self::ChannelClosed
    }
}

impl IntoResponse for AlertmanagerWebhookHandlerError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match self {
            Self::ChannelClosed => StatusCode::INTERNAL_SERVER_ERROR,
            Self::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::UnexpectedVersion(_) => StatusCode::BAD_REQUEST,
        };

        (status_code, Json(self)).into_response()
    }
}
