-- Add migration script here

CREATE TABLE IF NOT EXISTS alerts
(
    id              INTEGER       PRIMARY KEY AUTOINCREMENT,
    text            TEXT          NOT NULL,
    resolved        BOOLEAN       NOT NULL DEFAULT false,
    fingerprint     TEXT          DEFAULT NULL,
    notebook_id     TEXT          DEFAULT NULL,
    chart_filename  TEXT          DEFAULT NULL,
    slack_channel   TEXT          DEFAULT NULL,
    slack_ts        TEXT          DEFAULT NULL,
    sloth_service   TEXT          DEFAULT NULL,
    sloth_slo       TEXT          DEFAULT NULL,
    objective_name  TEXT          DEFAULT NULL,
    created_at      TIMESTAMP     NOT NULL,
    updated_at      TIMESTAMP     NOT NULL
);

CREATE UNIQUE INDEX alerts_fingerprint ON alerts(fingerprint);
