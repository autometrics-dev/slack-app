use super::alertmanager::handlers::receive_alertmanager_webhook;
use super::charts::handlers::charts_get;
use super::metrics::metrics_get;
use super::GlobalState;
use crate::service::Service;
use axum::routing::{get, post};
use axum::Router;

pub fn create_router(service: Service) -> Router<()> {
    let db = service.db.clone();

    let router = Router::new()
        .route("/", get(|| async { "No slackin'!" }))
        .route("/healthz", get(|| async { "healthy" }))
        .route("/metrics", get(metrics_get))
        .route("/api/alerts", post(receive_alertmanager_webhook))
        .route("/api/chart/:alert_id", get(charts_get));

    let state = GlobalState { db, service };

    router.with_state(state)
}
