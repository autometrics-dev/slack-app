mod errors;
#[cfg(test)]
mod tests;
mod timeseries;
mod types;

pub use errors::PrometheusServiceError;

use fiberplane::models::providers::Timeseries;
use fiberplane::models::timestamps::TimeRange;
use timeseries::{query_series, TimeseriesQuery};
use url::Url;

#[derive(clap::Args, Debug)]
pub struct PrometheusServiceConfig {
    /// Base URL on which Prometheus can be reached.
    #[clap(long, env, default_value = "http://localhost:9090/prometheus")]
    pub prometheus_url: Url,
}

#[cfg(test)]
impl PrometheusServiceConfig {
    pub fn new_test_config() -> Self {
        Self {
            prometheus_url: Url::parse("http://localhost:9090/prometheus").unwrap(),
        }
    }
}

pub struct PrometheusService {
    config: PrometheusServiceConfig,
}

impl PrometheusService {
    pub fn new(config: PrometheusServiceConfig) -> Self {
        Self { config }
    }

    pub async fn query_slo_timeseries(
        &self,
        slo: &str,
        objective_name: &str,
        time_range: TimeRange,
    ) -> Result<Vec<Timeseries>, PrometheusServiceError> {
        let query = query_for_slo(slo, objective_name, &time_range)?;

        let timeseries_query = TimeseriesQuery { query, time_range };

        query_series(timeseries_query, &self.config).await
    }
}

fn query_for_slo(
    slo: &str,
    objective_name: &str,
    time_range: &TimeRange,
) -> Result<String, PrometheusServiceError> {
    let interval = get_prometheus_window_from_time_range(time_range);
    // Should be calculated based on build_info, but for now we just use a fixed interval
    let build_info_interval = "5s".to_string();

    if let Some(percentile) = slo.strip_prefix("success-rate-") {
        Ok(format!(
            r#"
(
    sum by(function, module, version, commit, service_name) (
        rate(
            {{
                __name__=~"function_calls(_count)?(_total)?",
                result="ok", 
                objective_name="{objective_name}",
                objective_percentile="{percentile}"
            }}[{interval}]
        )
        * on (instance, job) group_left(version, commit) (
            last_over_time(build_info[{build_info_interval}])
            or on (instance, job) up
        )
    )
) / (
    sum by(function, module, version, commit, service_name) (
        rate(
            {{
                __name__=~"function_calls(_count)?(_total)?",
                objective_name="{objective_name}",
                objective_percentile="{percentile}"

            }}[{interval}]
        )
        * on (instance, job) group_left(version, commit) (
            last_over_time(build_info[{build_info_interval}])
            or on (instance, job) up
        )
    ) > 0
)
            "#
        ))
    } else if let Some(percentile) = slo.strip_prefix("latency-") {
        if let Ok(promql_percentile) =
            translate_objective_percentile_to_promql_percentile(percentile)
        {
            // NOTE - this query needs the latency value as well
            Ok(format!(
                r#"
label_replace(
    histogram_quantile(
        {promql_percentile},
        sum by (le, function, module, commit, version, service_name) (
        rate({{
            __name__=~"function_calls_duration(_seconds)?_bucket",
            objective_name="{objective_name}",
            objective_percentile="{percentile}",
        }}[{interval}])
        # Attach the version and commit labels from the build_info metric
        * on (instance, job) group_left(version, commit) (
            last_over_time(build_info[{build_info_interval}])
            or on (instance, job) up
        )
        )
    ),
    # Add the percentile_latency label to the time series
    "percentile_latency", "{percentile}", "", ""
)
        "#
            ))
        } else {
            Err(PrometheusServiceError::InvalidPercentile(
                percentile.to_owned(),
            ))
        }
    } else {
        Err(PrometheusServiceError::UnknownSlo(slo.to_owned()))
    }
}

fn get_prometheus_window_from_time_range(time_range: &TimeRange) -> String {
    let from = time_range.from.unix_timestamp();
    let to = time_range.to.unix_timestamp();

    format!("{}s", to - from)
}

fn translate_objective_percentile_to_promql_percentile(
    percentile: &str,
) -> Result<String, PrometheusServiceError> {
    match percentile.parse::<f64>() {
        Ok(percentile) => {
            let promql_percentile = percentile / 100.0;

            Ok(format!("{:.3}", promql_percentile)
                .trim_end_matches('0')
                .to_owned())
        }
        Err(_) => Err(PrometheusServiceError::InvalidPercentile(
            percentile.to_owned(),
        )),
    }
}
