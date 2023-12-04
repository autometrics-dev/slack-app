use super::{models::*, DbError};
use autometrics::autometrics;
use sqlx::{sqlite, types::time::OffsetDateTime};
use tracing::{instrument, trace};

#[derive(Clone)]
pub struct Db {
    pool: sqlite::SqlitePool,
}

#[autometrics]
impl Db {
    pub fn new(pool: sqlite::SqlitePool) -> Db {
        Db { pool }
    }

    #[instrument(skip(self, tx))]
    pub async fn alert_create(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        new_alert: NewAlert,
    ) -> Result<Alert, DbError> {
        let now = OffsetDateTime::now_utc();
        let alert = sqlx::query_as(
            "INSERT INTO alerts ( text, resolved, fingerprint, notebook_id, chart_filename, slack_channel, slack_ts, sloth_slo, sloth_service, objective_name, created_at, updated_at )
             VALUES ( $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12 )
             RETURNING *",
        )
        .bind(&new_alert.text)
        .bind(new_alert.resolved)
        .bind(new_alert.fingerprint.as_ref())
        .bind(new_alert.notebook_id.as_ref())
        .bind(new_alert.chart_filename.as_ref())
        .bind(new_alert.slack_channel.as_ref())
        .bind(new_alert.slack_ts.as_ref())
        .bind(new_alert.sloth_slo.as_ref())
        .bind(new_alert.sloth_service.as_ref())
        .bind(new_alert.objective_name.as_ref())
        .bind(now)
        .bind(now)
        .fetch_one(&mut **tx)
        .await?;

        Ok(alert)
    }

    #[instrument(skip(self, tx))]
    pub async fn alert_get(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        alert_id: i64,
    ) -> Result<Alert, DbError> {
        let alert = sqlx::query_as(
            "SELECT *
             FROM alerts
             WHERE id = $1",
        )
        .bind(alert_id)
        .fetch_one(&mut **tx)
        .await?;

        Ok(alert)
    }

    #[instrument(skip(self, tx))]
    pub async fn alert_get_by_fingerprint(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        fingerprint: &str,
    ) -> Result<Option<Alert>, DbError> {
        let alert = sqlx::query_as(
            "SELECT id, text, resolved, fingerprint, notebook_id, chart_filename, slack_channel, slack_ts, sloth_slo, sloth_service, objective_name, created_at, updated_at
             FROM alerts
             WHERE fingerprint = $1",
        )
        .bind(fingerprint)
        .fetch_optional(&mut **tx)
        .await?;

        Ok(alert)
    }

    #[instrument(skip(self, tx))]
    pub async fn alert_update(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        alert: &Alert,
    ) -> Result<(), DbError> {
        let result = sqlx::query(
            "UPDATE alerts
             SET resolved = $1, notebook_id = $2, slack_channel = $3, slack_ts = $4, chart_filename = $5, updated_at = $6
             WHERE id = $7",
        )
        .bind(alert.resolved)
        .bind(alert.notebook_id.as_ref())
        .bind(alert.slack_channel.as_ref())
        .bind(alert.slack_ts.as_ref())
        .bind(alert.chart_filename.as_ref())
        .bind(OffsetDateTime::now_utc())
        .bind(alert.id)
        .execute(&mut **tx)
        .await?;

        match result.rows_affected() {
            0 => Err(DbError::NotFound),
            1 => Ok(()),
            _ => Err(DbError::UnknownError),
        }
    }

    #[instrument(skip(self))]
    pub async fn start_transaction(&self) -> Result<sqlx::Transaction<'_, sqlx::Sqlite>, DbError> {
        trace!("starting db transaction");
        self.pool.begin().await.map_err(|err| err.into())
    }

    #[instrument(skip(self, tx))]
    pub async fn commit(&self, tx: sqlx::Transaction<'_, sqlx::Sqlite>) -> Result<(), DbError> {
        trace!("committing db transaction");
        tx.commit().await.map_err(|err| err.into())
    }
}
