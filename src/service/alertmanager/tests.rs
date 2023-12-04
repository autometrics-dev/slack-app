use crate::db::models::Alert;
use crate::service::alertmanager::*;
use crate::testutil::*;
use axum::extract::State;
use axum::Json;
use sqlx::types::time::OffsetDateTime;

#[tokio::test]
async fn alerts_create_new_unresolved() {
    run_test(
        service_setup,
        service_cleanup,
        |ServiceContext { db, service }| async move {
            // arrange
            let now = OffsetDateTime::now_utc();
            let payload = AlertmanagerWebhookPayload {
                alerts: vec![AlertmanagerAlert {
                    fingerprint: "12345".to_owned(),
                    generator_url: Default::default(),
                    annotations: Default::default(),
                    labels: Default::default(),
                    status: AlertStatus::Firing,
                    starts_at: now,
                    ends_at: now,
                }],
                status: AlertStatus::Firing,
                version: "4".to_string(),
                ..Default::default()
            };

            // act
            handlers::receive_alertmanager_webhook(State(service.clone()), Json(payload))
                .await
                .expect("Error receiving firing alert");

            let mut tx = db.start_transaction().await.unwrap();
            let alert = db.alert_get_by_fingerprint(&mut tx, "12345").await.unwrap();
            tx.commit().await.unwrap();

            // assert
            assert_matches!(
                alert,
                Some(Alert {
                    resolved,
                    ..
                }) if !resolved
            );
        },
    )
    .await;
}

#[tokio::test]
async fn alerts_create_new_resolved() {
    run_test(
        service_setup,
        service_cleanup,
        |ServiceContext { db, service }| async move {
            // arrange
            let now = OffsetDateTime::now_utc();
            let payload = AlertmanagerWebhookPayload {
                alerts: vec![AlertmanagerAlert {
                    fingerprint: "23456".to_owned(),
                    generator_url: Default::default(),
                    annotations: Default::default(),
                    labels: Default::default(),
                    status: AlertStatus::Resolved,
                    starts_at: now,
                    ends_at: now,
                }],
                status: AlertStatus::Resolved,
                version: "4".to_string(),
                ..Default::default()
            };

            // act
            handlers::receive_alertmanager_webhook(State(service.clone()), Json(payload))
                .await
                .expect("Error receiving resolved alert");

            let mut tx = db.start_transaction().await.unwrap();
            let alert = db.alert_get_by_fingerprint(&mut tx, "23456").await.unwrap();
            tx.commit().await.unwrap();

            // assert
            assert_matches!(
                alert,
                Some(Alert {
                    resolved,
                    ..
                }) if resolved
            );
        },
    )
    .await;
}

#[tokio::test]
async fn alerts_create_and_update() {
    run_test(
        service_setup,
        service_cleanup,
        |ServiceContext { db, service }| async move {
            // arrange
            let now = OffsetDateTime::now_utc();
            let mut payload = AlertmanagerWebhookPayload {
                alerts: vec![AlertmanagerAlert {
                    fingerprint: "34567".to_owned(),
                    generator_url: Default::default(),
                    annotations: Default::default(),
                    labels: Default::default(),
                    status: AlertStatus::Firing,
                    starts_at: now,
                    ends_at: now,
                }],
                status: AlertStatus::Firing,
                version: "4".to_string(),
                ..Default::default()
            };

            handlers::receive_alertmanager_webhook(State(service.clone()), Json(payload.clone()))
                .await
                .expect("Error receiving original alert");

            payload.alerts[0].status = AlertStatus::Resolved;

            // act
            handlers::receive_alertmanager_webhook(State(service.clone()), Json(payload))
                .await
                .expect("Error receiving updated alert");

            let mut tx = db.start_transaction().await.unwrap();
            let alert = db.alert_get_by_fingerprint(&mut tx, "34567").await.unwrap();
            tx.commit().await.unwrap();

            // assert
            assert_matches!(
                alert,
                Some(Alert {
                    resolved,
                    ..
                }) if resolved
            );
        },
    )
    .await;
}

#[test]
fn test_alert_text_instance_down() {
    use super::create_alert_text;

    // arrange
    let now = OffsetDateTime::now_utc();
    let payload = AlertmanagerWebhookPayload {
        alerts: vec![AlertmanagerAlert {
            fingerprint: "34567".to_owned(),
            generator_url: Default::default(),
            annotations: Default::default(),
            labels: BTreeMap::from([
                ("alertname".to_owned(), "InstanceDown".to_owned()),
                ("kubernetes_pod_name".to_owned(), "fluentd-1234".to_owned()),
                ("environment".to_owned(), "production".to_owned()),
            ]),
            status: Default::default(),
            starts_at: now,
            ends_at: now,
        }],
        version: "4".to_string(),
        ..Default::default()
    };

    // act
    let text = create_alert_text(&payload.alerts[0], &payload);

    // assert
    assert_eq!(
        text,
        "Instance \"fluentd-1234\" down [environment=production]"
    );
}

#[test]
fn test_alert_text_success_rate() {
    use super::create_alert_text;

    // arrange
    let now = OffsetDateTime::now_utc();
    let payload = AlertmanagerWebhookPayload {
        alerts: vec![AlertmanagerAlert {
            fingerprint: "34567".to_owned(),
            generator_url: Default::default(),
            annotations: Default::default(),
            labels: BTreeMap::from([
                ("category".to_owned(), "success-rate".to_owned()),
                ("sloth_service".to_owned(), "api".to_owned()),
                ("environment".to_owned(), "production".to_owned()),
            ]),
            status: Default::default(),
            starts_at: now,
            ends_at: now,
        }],
        version: "4".to_string(),
        ..Default::default()
    };

    // act
    let text = create_alert_text(&payload.alerts[0], &payload);

    // assert
    assert_eq!(text, "High Error Rate for \"api\" [environment=production]");
}

#[test]
fn test_alert_text_unknown_slo() {
    use super::create_alert_text;

    // arrange
    let now = OffsetDateTime::now_utc();
    let payload = AlertmanagerWebhookPayload {
        alerts: vec![AlertmanagerAlert {
            fingerprint: "34567".to_owned(),
            generator_url: Default::default(),
            annotations: Default::default(),
            labels: BTreeMap::from([
                ("sloth_slo".to_owned(), "other".to_owned()),
                ("sloth_service".to_owned(), "slack-app".to_owned()),
                ("environment".to_owned(), "dev".to_owned()),
            ]),
            status: Default::default(),
            starts_at: now,
            ends_at: now,
        }],
        version: "4".to_string(),
        ..Default::default()
    };

    // act
    let text = create_alert_text(&payload.alerts[0], &payload);

    // assert
    assert_eq!(
        text,
        "SLO \"other\" in danger for \"slack-app\" [environment=dev]"
    );
}
