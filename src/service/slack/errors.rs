use serde::Serialize;
use slack_morphism::errors::SlackClientError;
use thiserror::Error;

#[derive(Debug, Serialize, Error)]
#[serde(tag = "error", content = "details", rename_all = "snake_case")]
pub enum SlackServiceError {
    #[error("Config error: {0}")]
    Client(String),

    #[error("Cannot update message without timestamp")]
    MissingTimestamp,
}

impl From<SlackClientError> for SlackServiceError {
    fn from(error: SlackClientError) -> Self {
        Self::Client(error.to_string())
    }
}
