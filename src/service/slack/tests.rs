use super::build_message;
use crate::db::models::Alert;
use once_cell::sync::Lazy;
use time::OffsetDateTime;
use url::Url;

static SERVICE_URL: Lazy<Url> = Lazy::new(|| Url::parse("http://localhost:3031").unwrap());
static PROMETHEUS_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("http://localhost:9090/prometheus").unwrap());
static EXPLORER_URL: Lazy<Url> = Lazy::new(|| Url::parse("http://explorer.pmmp.dev").unwrap());

#[test]
fn test_firing_alert_message() {
    let now = OffsetDateTime::UNIX_EPOCH;
    let alert = Alert {
        id: 1234,
        text: "High Error Rate for \"api\" [environment=production]".to_owned(),
        resolved: false,
        fingerprint: None,
        notebook_id: None,
        chart_filename: None,
        sloth_service: None,
        sloth_slo: None,
        objective_name: Some("api".to_owned()),
        slack_channel: None,
        slack_ts: None,
        created_at: now,
        updated_at: now,
    };

    let message =
        build_message(&SERVICE_URL, &PROMETHEUS_URL, Some(&EXPLORER_URL), &alert).unwrap();

    insta::assert_yaml_snapshot!(message);
}

#[test]
fn test_resolved_alert_message() {
    let now = OffsetDateTime::UNIX_EPOCH;
    let alert = Alert {
        id: 1234,
        text: "High Error Rate for \"api\" [environment=production]".to_owned(),
        resolved: true,
        fingerprint: None,
        notebook_id: None,
        chart_filename: None,
        sloth_service: None,
        sloth_slo: None,
        objective_name: Some("api".to_owned()),
        slack_channel: None,
        slack_ts: None,
        created_at: now,
        updated_at: now,
    };

    let message =
        build_message(&SERVICE_URL, &PROMETHEUS_URL, Some(&EXPLORER_URL), &alert).unwrap();

    insta::assert_yaml_snapshot!(message);
}

#[test]
fn test_alert_message_with_chart() {
    let now = OffsetDateTime::UNIX_EPOCH;
    let alert = Alert {
        id: 1234,
        text: "High Error Rate for \"api\" [environment=production]".to_owned(),
        resolved: true,
        fingerprint: None,
        notebook_id: None,
        chart_filename: Some("1234.png".to_owned()),
        sloth_service: None,
        sloth_slo: None,
        objective_name: Some("api".to_owned()),
        slack_channel: None,
        slack_ts: None,
        created_at: now,
        updated_at: now,
    };

    let message =
        build_message(&SERVICE_URL, &PROMETHEUS_URL, Some(&EXPLORER_URL), &alert).unwrap();

    insta::assert_yaml_snapshot!(message);
}

#[test]
fn test_alert_message_with_explorer_button() {
    let now = OffsetDateTime::UNIX_EPOCH;
    let alert = Alert {
        id: 1234,
        text: "High Error Rate for \"api\" [environment=production]".to_owned(),
        resolved: true,
        fingerprint: None,
        notebook_id: None,
        chart_filename: Some("1234.png".to_owned()),
        sloth_service: Some("api".to_owned()),
        sloth_slo: Some("success-rate-99".to_owned()),
        objective_name: Some("api".to_owned()),
        slack_channel: None,
        slack_ts: None,
        created_at: now,
        updated_at: now,
    };

    let message =
        build_message(&SERVICE_URL, &PROMETHEUS_URL, Some(&EXPLORER_URL), &alert).unwrap();

    insta::assert_yaml_snapshot!(message);
}
