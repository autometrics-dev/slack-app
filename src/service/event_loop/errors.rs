use crate::db::DbError;
use crate::events::Event;
use crate::service::{ChartServiceError, SlackServiceError};
use serde::Serialize;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

#[derive(Debug, Error, Serialize)]
#[serde(tag = "error", content = "details", rename_all = "snake_case")]
pub enum EventLoopError {
    #[error("Channel unexpectedly closed")]
    ChannelClosed,

    #[error("Chart error: {0}")]
    ChartError(ChartServiceError),

    #[error("Database error: {0}")]
    DatabaseError(DbError),

    #[error("Entity not found")]
    NotFound,

    #[error("Slack error: {0}")]
    SlackError(SlackServiceError),
}

impl From<ChartServiceError> for EventLoopError {
    fn from(error: ChartServiceError) -> Self {
        Self::ChartError(error)
    }
}

impl From<DbError> for EventLoopError {
    fn from(error: DbError) -> Self {
        match error {
            DbError::NotFound => EventLoopError::NotFound,
            error => EventLoopError::DatabaseError(error),
        }
    }
}

impl From<SendError<Event>> for EventLoopError {
    fn from(_error: SendError<Event>) -> Self {
        Self::ChannelClosed
    }
}

impl From<SlackServiceError> for EventLoopError {
    fn from(error: SlackServiceError) -> Self {
        Self::SlackError(error)
    }
}
