mod alertmanager;
mod charts;
mod metrics;
mod prometheus;
mod slack;

pub mod event_loop;
pub mod router;

use crate::db::Db;
use crate::events::Event;
use autometrics::objectives::{Objective, ObjectiveLatency, ObjectivePercentile};
use axum::extract::FromRef;
use charts::ChartService;
use prometheus::PrometheusService;
use slack::SlackService;
use std::sync::{atomic::AtomicBool, Arc};
use tokio::sync::mpsc::Sender;
use url::Url;

pub use charts::{ChartServiceConfig, ChartServiceError};
pub use prometheus::{PrometheusServiceConfig, PrometheusServiceError};
pub use slack::{SlackServiceConfig, SlackServiceError};

pub const SLACK_APP_SLO: Objective = Objective::new("slack_app")
    .success_rate(ObjectivePercentile::P99)
    .latency(ObjectiveLatency::Ms250, ObjectivePercentile::P95);

#[derive(Clone, FromRef)]
pub struct GlobalState {
    pub db: Db,
    pub service: Service,
}

#[derive(Clone)]
pub struct Service {
    charts: Arc<ChartService>,
    db: Db,
    event_sender: Sender<Event>,
    prometheus: Arc<PrometheusService>,
    shutdown: Arc<AtomicBool>,
    slack: Arc<SlackService>,
}

impl Service {
    pub fn new(
        service_base_url: Url,
        chart_config: ChartServiceConfig,
        db: Db,
        event_sender: Sender<Event>,
        explorer_base_url: Option<Url>,
        prometheus_config: PrometheusServiceConfig,
        slack_config: SlackServiceConfig,
    ) -> Self {
        let prometheus_url = prometheus_config.prometheus_url.clone();
        Self {
            charts: Arc::new(ChartService::new(chart_config)),
            db,
            event_sender,
            prometheus: Arc::new(PrometheusService::new(prometheus_config)),
            shutdown: Arc::new(AtomicBool::new(false)),
            slack: Arc::new(SlackService::new(
                service_base_url,
                slack_config,
                prometheus_url,
                explorer_base_url,
            )),
        }
    }
}
