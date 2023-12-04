use crate::db::models::Alert;

#[derive(Debug)]
pub enum Event {
    /// Fetches data from Prometheus and generates a chart for the given alert.
    ///
    /// The chart is stored to disk, and once saved, it follows up with a
    /// `PostSlackAlert` event.
    ///
    /// If chart generation fails for whatever reason, it continues posting to
    /// Slack without a chart.
    CreateChartAndPostToSlack { alert: Alert },

    /// Posts a new Slack message for the given alert.
    PostSlackAlert { alert: Alert },

    /// Fetches the alert with the given ID from the DB, and updates the
    /// corresponding Slack message if its timestamp is known.
    UpdateSlackAlert { alert_id: i64 },

    /// Shuts down the service.
    Shutdown,
}
