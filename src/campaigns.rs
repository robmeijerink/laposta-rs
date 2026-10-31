//! Campaigns: creating, filling, scheduling and sending.

use std::collections::BTreeMap;

use futures_util::stream::BoxStream;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::client::{Body, Client, Query, decode, decode_list, decode_root};
use crate::error::{Error, Result};
use crate::history::{Event, History, HistoryQuery};
use crate::pagination;
use crate::serde_util::map_or_empty_array;
use crate::types::{AccountId, CampaignId, Cursor, Email, Limit, ListId, SegmentId, Timestamp};

/// A campaign.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct Campaign {
    /// Owning account.
    pub account_id: AccountId,
    /// Campaign id.
    pub campaign_id: CampaignId,
    /// Account timezone used for the datetime fields.
    pub timezone: String,
    /// Creation time.
    pub created: String,
    /// Creation time in ISO 8601 format.
    pub created_iso: String,
    /// Time of the last modification.
    pub modified: Option<String>,
    /// Time of the last modification in ISO 8601 format.
    pub modified_iso: Option<String>,
    /// Campaign type, currently always `regular`.
    #[serde(rename = "type")]
    pub kind: String,
    /// Requested delivery time.
    pub delivery_requested: Option<String>,
    /// Requested delivery time in ISO 8601 format.
    pub delivery_requested_iso: Option<String>,
    /// When delivery started.
    pub delivery_started: Option<String>,
    /// When delivery started, in ISO 8601 format.
    pub delivery_started_iso: Option<String>,
    /// When delivery ended.
    pub delivery_ended: Option<String>,
    /// When delivery ended, in ISO 8601 format.
    pub delivery_ended_iso: Option<String>,
    /// Internal name.
    pub name: String,
    /// Subject line.
    pub subject: String,
    /// Preview text shown by mail clients.
    pub previewtext: Option<String>,
    /// Sender.
    pub from: CampaignSender,
    /// Reply-to address.
    pub reply_to: Option<String>,
    /// Target lists, each with the segments selected within it.
    #[serde(deserialize_with = "map_or_empty_array")]
    pub list_ids: BTreeMap<ListId, Vec<SegmentId>>,
    /// Statistics settings, for example `{"ga": "true"}`.
    pub stats: Option<Value>,
    /// URL of the web version.
    pub web: Option<String>,
    /// Screenshot URLs keyed by size.
    #[serde(deserialize_with = "map_or_empty_array")]
    pub screenshot: Option<BTreeMap<String, String>>,
    /// `deleted` on a campaign that was just deleted.
    pub state: Option<String>,
}

/// Sender of a received [`Campaign`].
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct CampaignSender {
    /// Display name.
    pub name: String,
    /// Email address.
    pub email: String,
}

/// Sender of a campaign you create or update.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct Sender {
    /// Display name.
    pub name: String,
    /// Email address. Must be an approved sender in the account.
    pub email: Email,
}

impl Sender {
    /// Creates a sender.
    pub fn new(name: impl Into<String>, email: Email) -> Self {
        Self {
            name: name.into(),
            email,
        }
    }
}

/// A list, or a segment of a list, a campaign is sent to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Audience {
    /// All active members of a list.
    List(ListId),
    /// The members of a segment.
    Segment(SegmentId),
}

impl From<ListId> for Audience {
    fn from(id: ListId) -> Self {
        Self::List(id)
    }
}

impl From<SegmentId> for Audience {
    fn from(id: SegmentId) -> Self {
        Self::Segment(id)
    }
}

/// Parameters for creating a campaign. The campaign has no content yet.
#[derive(Debug, Clone, Serialize)]
pub struct NewCampaign {
    #[serde(rename = "type")]
    kind: &'static str,
    name: String,
    subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    previewtext: Option<String>,
    from: Sender,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<Email>,
    list_ids: Vec<Audience>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stats: Option<Stats>,
}

#[derive(Debug, Clone, Serialize)]
struct Stats {
    ga: bool,
}

impl NewCampaign {
    /// Starts a regular campaign sent to at least one audience.
    pub fn new(
        name: impl Into<String>,
        subject: impl Into<String>,
        from: Sender,
        audience: impl Into<Audience>,
    ) -> Self {
        Self {
            kind: "regular",
            name: name.into(),
            subject: subject.into(),
            previewtext: None,
            from,
            reply_to: None,
            list_ids: vec![audience.into()],
            stats: None,
        }
    }

    /// Adds another list or segment to send to.
    pub fn also_to(mut self, audience: impl Into<Audience>) -> Self {
        self.list_ids.push(audience.into());
        self
    }

    /// Sets the preview text (at most 120 characters).
    pub fn previewtext(mut self, text: impl Into<String>) -> Self {
        self.previewtext = Some(text.into());
        self
    }

    /// Sets the reply-to address.
    pub fn reply_to(mut self, email: Email) -> Self {
        self.reply_to = Some(email);
        self
    }

    /// Enables or disables Google Analytics tracking.
    pub fn google_analytics(mut self, enabled: bool) -> Self {
        self.stats = Some(Stats { ga: enabled });
        self
    }
}

/// Parameters for updating a campaign. Only fields that are set are changed.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateCampaign {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    previewtext: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    from: Option<Sender>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<Email>,
    #[serde(skip_serializing_if = "Option::is_none")]
    list_ids: Option<Vec<Audience>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stats: Option<Stats>,
}

impl UpdateCampaign {
    /// Creates an update that changes nothing yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Renames the campaign.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Changes the subject line.
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Changes the preview text. An empty string removes it.
    pub fn previewtext(mut self, text: impl Into<String>) -> Self {
        self.previewtext = Some(text.into());
        self
    }

    /// Changes the sender.
    pub fn from(mut self, sender: Sender) -> Self {
        self.from = Some(sender);
        self
    }

    /// Changes the reply-to address.
    pub fn reply_to(mut self, email: Email) -> Self {
        self.reply_to = Some(email);
        self
    }

    /// Replaces the audiences.
    pub fn audiences(mut self, audiences: impl IntoIterator<Item = Audience>) -> Self {
        self.list_ids = Some(audiences.into_iter().collect());
        self
    }

    /// Enables or disables Google Analytics tracking.
    pub fn google_analytics(mut self, enabled: bool) -> Self {
        self.stats = Some(Stats { ga: enabled });
        self
    }
}

enum ContentSource {
    Html(String),
    ImportUrl(url::Url),
}

/// Content to fill a campaign with: either HTML or a URL to import it from.
pub struct CampaignContent {
    source: ContentSource,
    inline_css: Option<bool>,
}

impl CampaignContent {
    /// Fills the campaign with the given HTML.
    pub fn html(html: impl Into<String>) -> Self {
        Self {
            source: ContentSource::Html(html.into()),
            inline_css: None,
        }
    }

    /// Lets Laposta import the HTML found at the URL.
    pub fn import_url(url: url::Url) -> Self {
        Self {
            source: ContentSource::ImportUrl(url),
            inline_css: None,
        }
    }

    /// Inlines CSS into the HTML.
    pub fn inline_css(mut self, inline: bool) -> Self {
        self.inline_css = Some(inline);
        self
    }

    fn body(&self) -> Value {
        let mut body = match &self.source {
            ContentSource::Html(html) => json!({ "html": html }),
            ContentSource::ImportUrl(url) => json!({ "import_url": url }),
        };
        if let Some(inline) = self.inline_css {
            body["inline_css"] = json!(inline);
        }
        body
    }
}

/// The content of a campaign.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct Content {
    /// Campaign id.
    pub campaign_id: CampaignId,
    /// Plain text version.
    pub plaintext: Option<String>,
    /// HTML version.
    pub html: Option<String>,
    /// URL the HTML was imported from.
    pub import_url: Option<String>,
    /// Problems found while importing. Only present after [`Campaigns::set_content`].
    pub report: Option<ContentReport>,
}

/// Problems found while importing campaign content. None of them prevents sending.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct ContentReport {
    /// JavaScript was found.
    pub javascript: bool,
    /// Flash was found.
    pub flash: bool,
    /// No unsubscribe link was found.
    pub no_unsubscribe: bool,
    /// An empty unsubscribe link was found.
    pub empty_unsubscribe: bool,
    /// External CSS files that could not be loaded.
    pub css: Value,
    /// External images that could not be loaded.
    pub images: Value,
}

/// Operations on campaigns, obtained from [`Client::campaigns`].
#[derive(Debug, Clone, Copy)]
pub struct Campaigns<'a> {
    client: &'a Client,
}

fn validate_text(what: &str, value: &str, max: usize) -> Result<()> {
    if value.chars().count() > max {
        return Err(Error::InvalidInput(format!(
            "{what} is limited to {max} characters"
        )));
    }
    Ok(())
}

impl<'a> Campaigns<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Creates a campaign. Fill it with [`set_content`](Self::set_content) before sending.
    pub async fn create(&self, campaign: &NewCampaign) -> Result<Campaign> {
        validate_text("name", &campaign.name, 255)?;
        validate_text("subject", &campaign.subject, 255)?;
        validate_text("sender name", &campaign.from.name, 255)?;
        if let Some(text) = &campaign.previewtext {
            validate_text("previewtext", text, 120)?;
        }
        let body = Body::Form(crate::client::to_value(campaign)?);
        let response = self
            .client
            .send(Method::POST, &["campaign"], &Query::new(), body)
            .await?;
        decode(response, "campaign")
    }

    /// Fetches a campaign.
    pub async fn get(&self, campaign_id: &CampaignId) -> Result<Campaign> {
        let response = self
            .client
            .send(
                Method::GET,
                &["campaign", campaign_id.as_str()],
                &Query::new(),
                Body::None,
            )
            .await?;
        decode(response, "campaign")
    }

    /// Fetches all campaigns.
    pub async fn all(&self) -> Result<Vec<Campaign>> {
        let response = self
            .client
            .send(Method::GET, &["campaign"], &Query::new(), Body::None)
            .await?;
        decode_list(response, "campaign")
    }

    /// Updates a campaign.
    pub async fn update(
        &self,
        campaign_id: &CampaignId,
        update: &UpdateCampaign,
    ) -> Result<Campaign> {
        for (what, value, max) in [
            ("name", &update.name, 255),
            ("subject", &update.subject, 255),
            ("previewtext", &update.previewtext, 120),
        ] {
            if let Some(value) = value {
                validate_text(what, value, max)?;
            }
        }
        let body = Body::Form(crate::client::to_value(update)?);
        let response = self
            .client
            .send(
                Method::POST,
                &["campaign", campaign_id.as_str()],
                &Query::new(),
                body,
            )
            .await?;
        decode(response, "campaign")
    }

    /// Deletes a campaign. Sent campaigns can be restored for 180 days.
    pub async fn delete(&self, campaign_id: &CampaignId) -> Result<Campaign> {
        let response = self
            .client
            .send(
                Method::DELETE,
                &["campaign", campaign_id.as_str()],
                &Query::new(),
                Body::None,
            )
            .await?;
        decode(response, "campaign")
    }

    /// Fetches one page of a campaign's history.
    pub async fn history(&self, campaign_id: &CampaignId, query: &HistoryQuery) -> Result<History> {
        let mut query = query.clone();
        query.campaign = None;
        let response = self
            .client
            .send(
                Method::GET,
                &["campaign", campaign_id.as_str(), "history"],
                &query.to_query(None),
                Body::None,
            )
            .await?;
        decode_root(response)
    }

    /// Streams a campaign's complete history.
    pub fn history_stream(
        &self,
        campaign_id: &CampaignId,
        query: HistoryQuery,
    ) -> BoxStream<'static, Result<Event>> {
        let client = self.client.clone();
        let campaign_id = campaign_id.clone();
        pagination::items(move |cursor: Option<Cursor>| {
            let client = client.clone();
            let campaign_id = campaign_id.clone();
            let mut query = query.clone();
            query.cursor = cursor;
            query.limit.get_or_insert_with(Limit::default);
            async move {
                let history = Campaigns::new(&client)
                    .history(&campaign_id, &query)
                    .await?;
                Ok(history.into_page())
            }
        })
    }

    /// Fetches the content of a campaign.
    ///
    /// Only works for campaigns whose content was imported, not for campaigns
    /// built in the drag and drop editor.
    pub async fn content(&self, campaign_id: &CampaignId) -> Result<Content> {
        let response = self
            .client
            .send(
                Method::GET,
                &["campaign", campaign_id.as_str(), "content"],
                &Query::new(),
                Body::None,
            )
            .await?;
        decode(response, "campaign")
    }

    /// Fills a campaign with content.
    pub async fn set_content(
        &self,
        campaign_id: &CampaignId,
        content: &CampaignContent,
    ) -> Result<Content> {
        let response = self
            .client
            .send(
                Method::POST,
                &["campaign", campaign_id.as_str(), "content"],
                &Query::new(),
                Body::Form(content.body()),
            )
            .await?;
        decode(response, "campaign")
    }

    /// Sends a campaign immediately. A campaign that was sent before is only
    /// sent to addresses added since.
    pub async fn send(&self, campaign_id: &CampaignId) -> Result<Campaign> {
        self.action(campaign_id, "send", Body::None).await
    }

    /// Schedules a campaign for later delivery.
    pub async fn schedule(
        &self,
        campaign_id: &CampaignId,
        delivery: &Timestamp,
    ) -> Result<Campaign> {
        let body = Body::Form(json!({ "delivery_requested": delivery }));
        self.action(campaign_id, "schedule", body).await
    }

    /// Sends a test email of a campaign that has content and has not been sent.
    pub async fn send_test(&self, campaign_id: &CampaignId, email: &Email) -> Result<Campaign> {
        let body = Body::Form(json!({ "email": email }));
        self.action(campaign_id, "testmail", body).await
    }

    async fn action(&self, campaign_id: &CampaignId, action: &str, body: Body) -> Result<Campaign> {
        let response = self
            .client
            .send(
                Method::POST,
                &["campaign", campaign_id.as_str(), "action", action],
                &Query::new(),
                body,
            )
            .await?;
        decode(response, "campaign")
    }
}
