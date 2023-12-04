mod errors;

use super::Service;
use crate::db::models::Alert;
use crate::events::Event;
use crate::service::prometheus::PrometheusServiceError;
use crate::service::SlackServiceError;
use autometrics::autometrics;
use errors::EventLoopError;
use fiberplane::models::timestamps::{TimeRange, Timestamp};
use std::sync::atomic::Ordering;
use time::ext::NumericalDuration;
use tokio::sync::mpsc::Receiver;
use tracing::{error, instrument, warn};

type EventResult = Result<(), EventLoopError>;

/// Handle all received messages from the `event_reader` until a shutdown
/// event is received.
pub async fn handle_events(
    service: &mut Service,
    mut event_reader: Receiver<Event>,
) -> EventResult {
    loop {
        match event_reader.recv().await {
            Some(event) => {
                use Event::*;
                let result = match event {
                    CreateChartAndPostToSlack { alert } => {
                        handle_create_chart(service, alert).await
                    }
                    PostSlackAlert { alert } => handle_post_slack_alert(service, alert).await,
                    UpdateSlackAlert { alert_id } => {
                        handle_update_slack_alert(service, alert_id).await
                    }
                    Shutdown => {
                        handle_shutdown(service);
                        return Ok(());
                    }
                };

                if let Err(err) = result {
                    error!(?err, "Unable to process event message");
                }
            }
            None => return Err(EventLoopError::ChannelClosed),
        }
    }
}

#[autometrics]
#[instrument(err, skip(service))]
async fn handle_create_chart(service: &mut Service, mut alert: Alert) -> EventResult {
    match (alert.sloth_slo.as_ref(), alert.objective_name.as_ref()) {
        (Some(slo), Some(objective_name)) => {
            let created_at = Timestamp::from(alert.created_at);
            let time_range = TimeRange {
                from: created_at - 6.hours(),
                to: created_at,
            };

            match service
                .prometheus
                .query_slo_timeseries(slo, objective_name, time_range.clone())
                .await
            {
                Ok(timeseries_data) => {
                    // Not bothering to be too graceful about error handling for this one
                    // because it's not dependent on external services. If something goes wrong inside this function
                    // it's most likely a filesystem issue, which would be serious enough
                    // that we might want to escalate it anyway.
                    let filename = service
                        .charts
                        .create_and_store_chart(slo, time_range, timeseries_data)
                        .await?;

                    alert.chart_filename = Some(filename);

                    update_alert(service, alert.clone()).await?;
                }
                Err(PrometheusServiceError::UnknownSlo(_)) => {
                    // Continue without chart.
                }
                Err(err) => {
                    // TODO: Should we include information in the Slack alert,
                    //       to tell the user we couldn't query Prometheus?
                    // NOTE: The function has a tracing::instrument attribute,
                    //        so the spans attached to the error! call will already
                    //        have the Alert in its attributes in theory
                    error!(?err, "Could not query Prometheus");
                }
            };
        }
        _ => {
            // Continue without chart.
        }
    }

    service
        .event_sender
        .send(Event::PostSlackAlert { alert })
        .await?;

    Ok(())
}

#[autometrics]
#[instrument(err, skip(service))]
async fn handle_post_slack_alert(service: &mut Service, mut alert: Alert) -> EventResult {
    let (slack_channel, slack_ts) = service.slack.send_alert(&alert).await?;

    alert.slack_channel = Some(slack_channel.to_string());
    alert.slack_ts = Some(slack_ts.to_string());

    update_alert(service, alert).await
}

#[autometrics]
#[instrument(err, skip(service))]
async fn handle_update_slack_alert(service: &mut Service, alert_id: i64) -> EventResult {
    let mut tx = service.db.start_transaction().await?;

    let alert = service.db.alert_get(&mut tx, alert_id).await?;

    if alert.slack_ts.is_some() {
        service.slack.update_alert(&alert).await?;
    } else {
        Err(SlackServiceError::MissingTimestamp)?;
    }

    service.db.commit(tx).await?;

    Ok(())
}

#[autometrics]
#[instrument(skip_all)]
fn handle_shutdown(service: &mut Service) {
    service.shutdown.store(true, Ordering::Release);
}

#[autometrics]
#[instrument(err, skip(service))]
async fn update_alert(service: &mut Service, alert: Alert) -> EventResult {
    let mut tx = service.db.start_transaction().await?;

    service.db.alert_update(&mut tx, &alert).await?;

    service.db.commit(tx).await?;

    Ok(())
}
