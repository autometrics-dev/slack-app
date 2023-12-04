use super::{
    create_alert_text, get_label, AlertmanagerWebhookHandlerError, AlertmanagerWebhookPayload,
};
use crate::db::models::NewAlert;
use crate::events::Event;
use crate::service::{Service, SLACK_APP_SLO};
use autometrics::autometrics;
use axum::extract::{Json, State};
use tracing::{debug, instrument};

#[autometrics(objective = SLACK_APP_SLO)]
#[instrument(err, skip(service))]
pub async fn receive_alertmanager_webhook(
    State(service): State<Service>,
    Json(payload): Json<AlertmanagerWebhookPayload>,
) -> Result<String, AlertmanagerWebhookHandlerError> {
    debug!(?payload, "Received alertmanager webhook");

    if payload.version != "4" {
        return Err(AlertmanagerWebhookHandlerError::UnexpectedVersion(
            payload.version,
        ));
    }

    let mut tx = service.db.start_transaction().await?;

    for alert in &payload.alerts {
        let existing_alert = service
            .db
            .alert_get_by_fingerprint(&mut tx, &alert.fingerprint)
            .await?;

        if let Some(mut existing_alert) = existing_alert {
            let resolved = alert.status.is_resolved();
            if existing_alert.resolved == resolved {
                continue;
            }

            existing_alert.resolved = resolved;

            service.db.alert_update(&mut tx, &existing_alert).await?;

            service
                .event_sender
                .send(Event::UpdateSlackAlert {
                    alert_id: existing_alert.id,
                })
                .await?;
        } else {
            let new_alert = NewAlert {
                text: create_alert_text(alert, &payload),
                resolved: alert.status.is_resolved(),
                fingerprint: Some(alert.fingerprint.clone()),
                chart_filename: None, // Will be filled in later, if applicable.
                notebook_id: None,
                slack_channel: None, // Will be filled in later, once posted.
                slack_ts: None,      // Will be filled in later, once posted.
                sloth_slo: get_label(alert, &payload, "sloth_slo").map(str::to_owned),
                sloth_service: get_label(alert, &payload, "sloth_service").map(str::to_owned),
                objective_name: get_label(alert, &payload, "objective_name").map(str::to_owned),
                severity: get_label(alert, &payload, "severity").map(str::to_owned),
            };

            let db_alert = service.db.alert_create(&mut tx, new_alert).await?;

            service
                .event_sender
                .send(Event::CreateChartAndPostToSlack { alert: db_alert })
                .await?;
        }
    }

    service.db.commit(tx).await?;

    Ok("ok".to_owned())
}
