use crate::db::DbError;
use crate::service::{Service, SLACK_APP_SLO};
use autometrics::autometrics;
use axum::body::StreamBody;
use axum::extract::{Json, Path, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde::Serialize;
use thiserror::Error;
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use tracing::instrument;

#[derive(Debug, Error, Serialize)]
#[serde(tag = "error", content = "details", rename_all = "snake_case")]
pub enum ChartHandlerError {
    #[error("Database error: {0}")]
    Database(DbError),

    #[error("Cannot read file: {0}")]
    FileError(String),

    #[error("Entity not found")]
    NotFound,
}

impl From<DbError> for ChartHandlerError {
    fn from(error: DbError) -> Self {
        match error {
            DbError::NotFound => ChartHandlerError::NotFound,
            error => ChartHandlerError::Database(error),
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
            Self::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::FileError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::NotFound => StatusCode::NOT_FOUND,
        };

        (status_code, Json(self)).into_response()
    }
}

#[autometrics(objective = SLACK_APP_SLO)]
#[instrument(err, skip(service))]
pub async fn charts_get(
    State(service): State<Service>,
    Path(alert_id): Path<i64>,
) -> Result<impl IntoResponse, ChartHandlerError> {
    let mut tx = service.db.start_transaction().await?;

    let alert = service.db.alert_get(&mut tx, alert_id).await?;

    service.db.commit(tx).await?;

    let Some(filename) = alert.chart_filename.as_ref() else {
        return Err(ChartHandlerError::NotFound);
    };

    let headers = HeaderMap::from_iter([(CONTENT_TYPE, "image/png".parse().unwrap())]);

    let file: File = File::open(service.charts.config.storage_dir.join(filename)).await?;
    let stream = ReaderStream::new(file);
    let body = StreamBody::new(stream);

    Ok((headers, body))
}
