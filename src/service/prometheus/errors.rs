use serde::Serialize;
use std::num::ParseFloatError;
use thiserror::Error;

#[derive(Debug, Serialize, Error)]
#[serde(tag = "error", content = "details", rename_all = "snake_case")]
pub enum PrometheusServiceError {
    #[error("Config error: {0}")]
    Config(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("HTTP request error: {0}")]
    Http(String),

    #[error("Percentile error: {0}")]
    InvalidPercentile(String),

    #[error("Unrecognized SLO: {0}")]
    UnknownSlo(String),
}

impl From<ParseFloatError> for PrometheusServiceError {
    fn from(error: ParseFloatError) -> Self {
        Self::Deserialization(format!("Could not parse number: {error}"))
    }
}
