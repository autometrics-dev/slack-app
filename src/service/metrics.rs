use autometrics::prometheus_exporter;
use axum::response::IntoResponse;

pub async fn metrics_get() -> impl IntoResponse {
    prometheus_exporter::encode_http_response()
}
