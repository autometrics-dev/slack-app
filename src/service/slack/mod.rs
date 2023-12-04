mod errors;
#[cfg(test)]
mod tests;

use crate::db::models::Alert;
use fiberplane::models::timestamps::Timestamp;
use secrecy::{ExposeSecret, SecretString};
use slack_morphism::prelude::*;
use time::ext::NumericalDuration;
use url::Url;

pub use errors::SlackServiceError;

#[derive(clap::Args, Debug)]
pub struct SlackServiceConfig {
    /// Slack channel to post to.
    /// TODO: This should be a user setting.
    #[clap(
        long = "slack-channel",
        env = "SLACK_CHANNEL",
        required = true,
        help_heading = "Slack options"
    )]
    channel: String,

    /// Slack bot token, which is unique per workspace in which the app is
    /// installed. This means we still need to provide a way for the user to
    /// set this token in their FMP configuration.
    ///
    /// Additionally, the token is tied to a Slack client ID and secret, which
    /// will be the same ID and secret as used for our main API service. This
    /// makes sense from a user perspective, since there will only be a single
    /// Fiberplane app to install. It will create a challenge for us when we
    /// want to handle incoming events from Slack however, since those will
    /// always be delivered to the main API instead. By then, Slack app
    /// instances probably have to set up WebSocket connections to the API so
    /// that the API can deliver the events to the correct instance.
    #[clap(
        long = "slack-bot-token",
        env = "SLACK_BOT_TOKEN",
        required = true,
        help_heading = "Slack options"
    )]
    token: SecretString,
}

#[cfg(test)]
impl SlackServiceConfig {
    pub fn new_test_config(token: impl Into<SecretString>) -> Self {
        Self {
            channel: "test-channel".to_owned(),
            token: token.into(),
        }
    }
}

pub struct SlackService {
    /// Service URL of the Slack app itself.
    ///
    /// This is used by the Slack service to link to images included in
    /// messages.
    service_base_url: Url,

    /// Slack channel to post alerts to.
    channel: SlackChannelId,

    /// Slack client.
    client: SlackClient<SlackClientHyperHttpsConnector>,

    /// The API token for authenticating with Slack.
    token: SlackApiToken,

    /// URL of the Prometheus instance, used in links to Explorer.
    prometheus_url: Url,

    /// Optional URL where the Explorer is hosted.
    ///
    /// If a URL is provided, "Open in Explorer" buttons are added to messages
    /// for compatible alerts.
    explorer_base_url: Option<Url>,
}

impl SlackService {
    pub fn new(
        service_base_url: Url,
        config: SlackServiceConfig,
        prometheus_url: Url,
        explorer_base_url: Option<Url>,
    ) -> Self {
        let channel = SlackChannelId(config.channel.clone());
        let client = SlackClient::new(SlackClientHyperConnector::new());
        let token_value: SlackApiTokenValue = config.token.expose_secret().into();
        let token: SlackApiToken = SlackApiToken::new(token_value);

        Self {
            service_base_url,
            channel,
            client,
            prometheus_url,
            explorer_base_url,
            token,
        }
    }

    pub async fn send_alert(
        &self,
        alert: &Alert,
    ) -> Result<(SlackChannelId, SlackTs), SlackServiceError> {
        let post_message_request = SlackApiChatPostMessageRequest::new(
            self.channel.clone(),
            build_message(
                &self.service_base_url,
                &self.prometheus_url,
                self.explorer_base_url.as_ref(),
                alert,
            )?,
        );

        let response = self
            .client
            .open_session(&self.token)
            .chat_post_message(&post_message_request)
            .await?;

        Ok((response.channel, response.ts))
    }

    pub async fn update_alert(&self, alert: &Alert) -> Result<(), SlackServiceError> {
        let Some(ts) = alert.slack_ts.as_ref() else {
            return Err(SlackServiceError::MissingTimestamp);
        };

        let channel = alert
            .slack_channel
            .as_ref()
            .cloned()
            .map(SlackChannelId::new)
            .unwrap_or_else(|| self.channel.clone());

        let update_request = SlackApiChatUpdateRequest::new(
            channel,
            build_message(
                &self.service_base_url,
                &self.prometheus_url,
                self.explorer_base_url.as_ref(),
                alert,
            )?,
            ts.into(),
        )
        .with_as_user(true);

        self.client
            .open_session(&self.token)
            .chat_update(&update_request)
            .await?;

        Ok(())
    }
}

fn build_message(
    service_base_url: &Url,
    prometheus_url: &Url,
    explorer_url: Option<&Url>,
    alert: &Alert,
) -> Result<SlackMessageContent, SlackServiceError> {
    let color = if alert.resolved {
        // Green
        "#2EC95A".to_owned()
    } else {
        // Red
        "#F2303C".to_owned()
    };

    let header_text = if alert.resolved {
        ":white_check_mark: Alert was resolved".to_owned()
    } else {
        ":rotating_light: Alert is firing".to_owned()
    };

    let header_block = SlackSectionBlock::new().with_text(SlackBlockText::Plain(
        SlackBlockPlainText::new(header_text.into()).with_emoji(true),
    ));

    let severity_text = match alert.severity.as_deref() {
        Some("page") => ":pager: Page".to_owned(),
        Some("ticket") => ":ticket: Ticket".to_owned(),
        Some(severity) => format!(":question: {}", severity),
        None => ":question: Unknown".to_owned(),
    };

    let description_block = SlackSectionBlock::new()
        .with_text(SlackBlockMarkDownText::new(alert.text.clone().into()).into())
        .with_fields(vec![
            SlackBlockMarkDownText::new(format!("*Severity*\n{}", severity_text)).into(),
            SlackBlockMarkDownText::new(format!("*Created*\n{}", alert.created_at)).into(),
        ]);

    let chart_block = if let Some(_chart_filename) = alert.chart_filename.as_ref() {
        let section: SlackBlock = SlackImageBlock::new(
            service_base_url
                .join(&format!("/api/chart/{}", alert.id))
                .unwrap(),
            format!(
                "Chart for slo `{}`",
                alert.sloth_slo.as_deref().unwrap_or("unknown")
            ),
        )
        .into();
        Some(section)
    } else {
        None
    };

    let actions_block = if let Some(explorer_alert_url) =
        get_explorer_alert_url(explorer_url, prometheus_url, alert)
    {
        let description = format!(
            "Triage `{}` SLO in Explorer",
            alert.sloth_slo.as_deref().unwrap_or("unknown")
        );
        let explorer_block: SlackBlock = SlackSectionBlock::new()
            .with_text(SlackBlockText::MarkDown(SlackBlockMarkDownText::new(
                description.into(),
            )))
            .with_accessory(
                SlackBlockButtonElement::new("open_in_explorer".into(), "Open".into())
                    .with_url(explorer_alert_url)
                    .into(),
            )
            .into();
        Some(explorer_block)
    } else {
        None
    };

    let blocks_maybe: Vec<Option<SlackBlock>> = vec![
        Some(header_block.into()),
        Some(description_block.into()),
        chart_block,
        actions_block,
    ];
    let blocks: Vec<SlackBlock> = blocks_maybe.into_iter().filter_map(|block| block).collect();

    let attachment = SlackMessageAttachment::new()
        .with_color(color)
        .with_blocks(blocks);

    let content = SlackMessageContent::new().with_attachments(vec![attachment]);

    Ok(content)
}

/// Returns the URL to link to Explorer for a given alert.
fn get_explorer_alert_url(
    base_url: Option<&Url>,
    prometheus_url: &Url,
    alert: &Alert,
) -> Option<Url> {
    match (
        base_url,
        alert.sloth_slo.as_ref(),
        alert.objective_name.as_ref(),
    ) {
        (Some(base_url), Some(slo), Some(objective_name)) => {
            let metric = if slo.starts_with("success-rate-") {
                "successRate"
            } else if slo.starts_with("latency-") {
                "latency"
            } else {
                return None;
            };
            let from = Timestamp::from(alert.created_at - 6.hours()).to_string();
            let to = Timestamp::from(alert.created_at).to_string();

            let mut url = base_url.clone();
            url.set_query(Some(format!("prometheusUrl={}", prometheus_url).as_str()));
            url.set_fragment(Some(&format!(
                "/slos/{}/{}?from={}&to={}",
                objective_name, metric, from, to
            )));
            Some(url)
        }
        _ => None,
    }
}
