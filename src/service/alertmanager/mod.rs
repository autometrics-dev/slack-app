mod errors;
#[cfg(test)]
mod tests;

pub mod handlers;

use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use std::collections::BTreeMap;

pub use errors::AlertmanagerWebhookHandlerError;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertmanagerAlert {
    /// Fingerprint to identify the alert.
    fingerprint: String,

    /// Identifies the entity that caused the alert.
    #[serde(rename = "generatorURL")]
    generator_url: String,

    #[serde(default)]
    annotations: BTreeMap<String, String>,

    #[serde(default)]
    labels: BTreeMap<String, String>,

    status: AlertStatus,

    #[serde(with = "time::serde::rfc3339")]
    starts_at: OffsetDateTime,

    #[serde(with = "time::serde::rfc3339")]
    ends_at: OffsetDateTime,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertmanagerWebhookPayload {
    alerts: Vec<AlertmanagerAlert>,

    #[serde(default)]
    common_annotations: BTreeMap<String, String>,

    #[serde(default)]
    common_labels: BTreeMap<String, String>,

    /// Backlink to the Alertmanager.
    #[serde(default, rename = "externalURL")]
    external_url: String,

    /// Key identifying the group of alerts (e.g. to deduplicate).
    group_key: String,

    #[serde(default)]
    group_labels: BTreeMap<String, String>,

    #[serde(default)]
    receiver: String,

    #[serde(default)]
    status: AlertStatus,

    /// The amount of alerts that have been truncated due to the "max_alerts"
    /// setting.
    #[serde(default)]
    truncated_alerts: u32,

    /// Webhook protocol version.
    ///
    /// We always expect version "4".
    version: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertStatus {
    #[default]
    Firing,
    Resolved,
}

impl AlertStatus {
    fn is_resolved(&self) -> bool {
        matches!(self, Self::Resolved)
    }
}

fn create_alert_text(alert: &AlertmanagerAlert, payload: &AlertmanagerWebhookPayload) -> String {
    // Alerts created from Sloth have a "summary" annotation that we can use
    if let Some(summary) = alert.annotations.get("summary") {
        return summary.to_owned();
    }

    let get_label = |key| get_label(alert, payload, key);

    let mut text = match get_label("alertname") {
        Some("InstanceDown") => format!(
            "Instance \"{}\" down",
            get_label("kubernetes_pod_name")
                .or_else(|| get_label("instance"))
                .unwrap_or("unknown")
        ),
        _ => match get_label("category") {
            Some("success-rate") => "High Error Rate".to_owned(),
            Some("latency") => "High Latency".to_owned(),
            _ => match get_label("sloth_slo") {
                Some(slo) => format!("SLO \"{slo}\" in danger"),
                None => "Alert".to_owned(),
            },
        },
    };

    if let Some(service) = get_label("sloth_service") {
        text.push_str(" for \"");
        text.push_str(service);
        text.push('"');
    }

    if let Some(environment) = get_label("environment") {
        text.push_str(" [environment=");
        text.push_str(environment);
        text.push(']');
    }

    text
}

fn get_label<'a>(
    alert: &'a AlertmanagerAlert,
    payload: &'a AlertmanagerWebhookPayload,
    key: &'_ str,
) -> Option<&'a str> {
    alert
        .labels
        .get(key)
        .or_else(|| payload.common_labels.get(key))
        .map(String::as_str)
}
