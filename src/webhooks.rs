//! Webhook management and incoming notification handling.

use std::fmt;

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::client::{Body, Client, Query, decode, decode_list, to_value, with_field};
use crate::error::Result;
use crate::types::{ListId, WebhookId};

/// Member event a webhook reacts to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WebhookEvent {
    /// A member subscribed.
    Subscribed,
    /// A member was modified.
    Modified,
    /// A member was deactivated: unsubscribed, deleted or bounced.
    Deactivated,
    /// An event this crate does not know about. It cannot be sent.
    #[default]
    #[serde(other, skip_serializing)]
    Unknown,
}

/// Whether a webhook is in use or has been deleted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WebhookState {
    /// The webhook is active.
    Active,
    /// The webhook was deleted.
    Deleted,
    /// A state this crate does not know about.
    #[default]
    #[serde(other)]
    Unknown,
}

/// A webhook.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct Webhook {
    /// Webhook id.
    pub webhook_id: WebhookId,
    /// The list the webhook belongs to.
    pub list_id: ListId,
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
    /// Current state.
    pub state: WebhookState,
    /// The event that triggers the webhook.
    pub event: WebhookEvent,
    /// Where notifications are sent.
    pub url: String,
    /// Whether delivery is blocked, for example after repeated failures.
    pub blocked: bool,
    /// Secret used to sign notifications. See `verify_signature`.
    pub secret: Option<Secret>,
}

/// A secret value that is left out of `Debug` output.
#[derive(Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct Secret(String);

impl Secret {
    /// The secret itself. Treat it as sensitive.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret([redacted])")
    }
}

/// Parameters for creating a webhook.
#[derive(Debug, Clone, Serialize)]
pub struct NewWebhook {
    event: WebhookEvent,
    url: url::Url,
    blocked: bool,
}

impl NewWebhook {
    /// Creates a webhook that is not blocked.
    pub fn new(event: WebhookEvent, url: url::Url) -> Self {
        Self {
            event,
            url,
            blocked: false,
        }
    }

    /// Creates the webhook in blocked state.
    pub fn blocked(mut self, blocked: bool) -> Self {
        self.blocked = blocked;
        self
    }
}

/// Parameters for updating a webhook. Only fields that are set are changed.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateWebhook {
    #[serde(skip_serializing_if = "Option::is_none")]
    event: Option<WebhookEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<url::Url>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blocked: Option<bool>,
}

impl UpdateWebhook {
    /// Creates an update that changes nothing yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Changes the triggering event.
    pub fn event(mut self, event: WebhookEvent) -> Self {
        self.event = Some(event);
        self
    }

    /// Changes the destination.
    pub fn url(mut self, url: url::Url) -> Self {
        self.url = Some(url);
        self
    }

    /// Blocks or unblocks delivery.
    pub fn blocked(mut self, blocked: bool) -> Self {
        self.blocked = Some(blocked);
        self
    }
}

/// Operations on webhooks, obtained from [`Client::webhooks`].
#[derive(Debug, Clone, Copy)]
pub struct Webhooks<'a> {
    client: &'a Client,
}

impl<'a> Webhooks<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Creates a webhook for a list.
    pub async fn create(&self, list_id: &ListId, webhook: &NewWebhook) -> Result<Webhook> {
        let body = Body::Form(with_field(to_value(webhook)?, "list_id", list_id.as_str()));
        let response = self
            .client
            .send(Method::POST, &["webhook"], &Query::new(), body)
            .await?;
        decode(response, "webhook")
    }

    /// Fetches a webhook.
    pub async fn get(&self, list_id: &ListId, webhook_id: &WebhookId) -> Result<Webhook> {
        let query = Query::new().with("list_id", list_id);
        let response = self
            .client
            .send(
                Method::GET,
                &["webhook", webhook_id.as_str()],
                &query,
                Body::None,
            )
            .await?;
        decode(response, "webhook")
    }

    /// Fetches all webhooks of a list.
    pub async fn all(&self, list_id: &ListId) -> Result<Vec<Webhook>> {
        let query = Query::new().with("list_id", list_id);
        let response = self
            .client
            .send(Method::GET, &["webhook"], &query, Body::None)
            .await?;
        decode_list(response, "webhook")
    }

    /// Updates a webhook.
    pub async fn update(
        &self,
        list_id: &ListId,
        webhook_id: &WebhookId,
        update: &UpdateWebhook,
    ) -> Result<Webhook> {
        let body = Body::Form(with_field(to_value(update)?, "list_id", list_id.as_str()));
        let response = self
            .client
            .send(
                Method::POST,
                &["webhook", webhook_id.as_str()],
                &Query::new(),
                body,
            )
            .await?;
        decode(response, "webhook")
    }

    /// Permanently deletes a webhook.
    pub async fn delete(&self, list_id: &ListId, webhook_id: &WebhookId) -> Result<Webhook> {
        let query = Query::new().with("list_id", list_id);
        let response = self
            .client
            .send(
                Method::DELETE,
                &["webhook", webhook_id.as_str()],
                &query,
                Body::None,
            )
            .await?;
        decode(response, "webhook")
    }
}

#[cfg(feature = "webhook")]
pub use notification::{
    WebhookInfo, WebhookNotification, WebhookPayload, WebhookSource, parse_payload,
    verify_signature,
};

#[cfg(feature = "webhook")]
mod notification {
    use hmac::{Hmac, Mac};
    use serde::Deserialize;
    use sha2::Sha256;

    use super::WebhookEvent;
    use crate::error::{Error, Result};
    use crate::members::Member;

    /// Origin of the change that triggered a notification.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[non_exhaustive]
    pub enum WebhookSource {
        /// Changed in the Laposta application.
        App,
        /// Changed by an external source, such as a subscription form.
        External,
        /// Changed through the API.
        Api,
        /// Changed by an automation.
        Automation,
        /// A source this crate does not know about.
        #[default]
        #[serde(other)]
        Unknown,
    }

    /// Metadata of a [`WebhookNotification`].
    #[derive(Debug, Clone, Default, Deserialize)]
    #[serde(default)]
    #[non_exhaustive]
    pub struct WebhookInfo {
        /// When the change happened.
        pub date_event: String,
        /// What happened, for example `resubscribed`, `unsubscribed`, `deleted` or `hardbounce`.
        pub action: String,
        /// Where the change came from.
        pub source: WebhookSource,
    }

    /// A single member event inside a [`WebhookPayload`].
    #[derive(Debug, Clone, Default, Deserialize)]
    #[serde(default)]
    #[non_exhaustive]
    pub struct WebhookNotification {
        /// Object type, currently always `member`.
        #[serde(rename = "type")]
        pub kind: String,
        /// The event.
        pub event: WebhookEvent,
        /// The member as it is now.
        pub data: Member,
        /// Metadata about the change.
        pub info: WebhookInfo,
    }

    /// The body Laposta posts to a webhook URL.
    #[derive(Debug, Clone, Default, Deserialize)]
    #[serde(default)]
    #[non_exhaustive]
    pub struct WebhookPayload {
        /// The bundled events.
        pub data: Vec<WebhookNotification>,
        /// When the request was made.
        pub date_requested: String,
    }

    /// Checks the `X-Laposta-Signature` header against the raw request body.
    ///
    /// `secret` is [`Webhook::secret`](super::Webhook::secret). An empty secret never
    /// verifies. The comparison is constant time.
    ///
    /// The signature does not protect against replays. Compare
    /// [`WebhookPayload::date_requested`] with the current time if that matters.
    pub fn verify_signature(body: &[u8], header: &str, secret: &str) -> bool {
        if secret.is_empty() {
            return false;
        }
        let Some(signature) = header.trim().strip_prefix("sha256=") else {
            return false;
        };
        let Ok(expected) = hex::decode(signature) else {
            return false;
        };
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
            .expect("HMAC accepts keys of any length");
        mac.update(body);
        mac.verify_slice(&expected).is_ok()
    }

    /// Parses the body of a webhook request. Verify the signature first.
    pub fn parse_payload(body: &[u8]) -> Result<WebhookPayload> {
        serde_json::from_slice(body).map_err(|source| Error::Decode {
            source,
            body: String::from_utf8_lossy(body).chars().take(500).collect(),
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn sign(body: &[u8], secret: &str) -> String {
            let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
            mac.update(body);
            format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
        }

        #[test]
        fn accepts_valid_signature() {
            let header = sign(b"{}", "secret");
            assert!(verify_signature(b"{}", &header, "secret"));
        }

        #[test]
        fn rejects_tampered_body_wrong_secret_and_malformed_headers() {
            let header = sign(b"{}", "secret");
            assert!(!verify_signature(b"{ }", &header, "secret"));
            assert!(!verify_signature(b"{}", &header, "other"));
            assert!(!verify_signature(b"{}", "sha256=zz", "secret"));
            assert!(!verify_signature(b"{}", "md5=abcd", "secret"));
            assert!(!verify_signature(b"{}", "", "secret"));
        }
    }
}
