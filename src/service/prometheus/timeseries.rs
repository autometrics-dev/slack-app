use super::{types::*, PrometheusServiceConfig, PrometheusServiceError};
use fiberplane::models::providers::Timeseries;
use fiberplane::models::timestamps::{TimeRange, Timestamp};
use reqwest::header::CONTENT_TYPE;
use reqwest::Client;
use std::time::Duration;
use tracing::debug;

pub(crate) struct TimeseriesQuery {
    pub query: String,
    pub time_range: TimeRange,
}

pub(crate) async fn query_series(
    query: TimeseriesQuery,
    config: &PrometheusServiceConfig,
) -> Result<Vec<Timeseries>, PrometheusServiceError> {
    let from = to_float(query.time_range.from);
    let to = to_float(query.time_range.to);
    let step = step_for_range(from, to);

    let query_string = {
        let mut form_data = form_urlencoded::Serializer::new(String::new());
        form_data.append_pair("query", &query.query);
        form_data.append_pair("start", &query.time_range.from.to_string());
        form_data.append_pair("end", &query.time_range.to.to_string());
        form_data.append_pair("step", &step.to_string());
        form_data.finish()
    };

    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .expect("Error building reqwest client");

    let mut url = config.prometheus_url.clone();
    url.path_segments_mut()
        .map_err(|_| {
            PrometheusServiceError::Config(format!(
                "Cannot append to prometheus base URL: {}",
                config.prometheus_url
            ))
        })?
        .extend(&["api", "v1", "query_range"]);

    url.set_query(Some(&query_string));

    let url_str = url.as_str();
    debug!(?url_str, "Querying prometheus query_range api");

    let response = client
        .post(url)
        .body(query_string)
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .send()
        .await
        .map_err(|err| PrometheusServiceError::Http(err.to_string()))?;

    let response: PrometheusResponse = response.json().await.map_err(|err| {
        PrometheusServiceError::Deserialization(format!(
            "Could not deserialize Prometheus response: {err}"
        ))
    })?;

    let PrometheusData::Matrix(matrix) = response.data;

    matrix.into_iter().map(RangeVector::into_series).collect()
}

fn to_float(timestamp: Timestamp) -> f64 {
    timestamp.unix_timestamp_nanos() as f64 / 1_000_000_000.0
}

/// Returns the step to fetch from the given duration in seconds. We attempt
/// to maintain roughly 120 steps for whatever the duration is, so that for a
/// duration of one hour, we fetch per 30 seconds, however for a duration of one
/// minute, we fetch per 1 seconds (as the step value is rounded up to a
/// full unit).
fn step_for_range(from: f64, to: f64) -> StepSize {
    let mut step = (to - from) / 120.0;
    let mut unit = StepUnit::Seconds;
    if step >= 60.0 {
        step /= 60.0;
        unit = StepUnit::Minutes;
        if step >= 60.0 {
            step /= 60.0;
            unit = StepUnit::Hours;
        }
    }

    StepSize {
        amount: f64::ceil(step) as u32,
        unit,
    }
}
#[derive(Clone, Copy)]
struct StepSize {
    amount: u32,
    unit: StepUnit,
}

impl ToString for StepSize {
    fn to_string(&self) -> String {
        format!("{}{}", self.amount, self.unit.to_str())
    }
}

#[derive(Clone, Copy)]
enum StepUnit {
    Hours,
    Minutes,
    Seconds,
}

impl StepUnit {
    fn to_str(self) -> &'static str {
        match self {
            Self::Hours => "h",
            Self::Minutes => "m",
            Self::Seconds => "s",
        }
    }
}
