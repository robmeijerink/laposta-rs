//! Campaign results.

use reqwest::Method;
use serde::Deserialize;

use crate::client::{Body, Client, Query, decode, decode_list};
use crate::error::Result;
use crate::serde_util::{f64_or_string, u64_or_string};
use crate::types::{AccountId, CampaignId};

/// Results of a sent campaign.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct Report {
    /// Owning account.
    pub account_id: AccountId,
    /// Campaign id.
    pub campaign_id: CampaignId,
    /// Emails sent.
    #[serde(deserialize_with = "u64_or_string")]
    pub sent: u64,
    /// Emails accepted by the receiving server.
    #[serde(deserialize_with = "u64_or_string")]
    pub accepted: u64,
    /// Addresses removed from the list.
    #[serde(deserialize_with = "u64_or_string")]
    pub cleaned: u64,
    /// Spam complaints.
    #[serde(deserialize_with = "u64_or_string")]
    pub complained: u64,
    /// Hard bounces.
    #[serde(deserialize_with = "u64_or_string")]
    pub hardbounced: u64,
    /// Unsubscriptions.
    #[serde(deserialize_with = "u64_or_string")]
    pub unsubscribed: u64,
    /// Members that opened the email at least once.
    #[serde(deserialize_with = "u64_or_string")]
    pub opened_unique: u64,
    /// Members that clicked at least once.
    #[serde(deserialize_with = "u64_or_string")]
    pub clicked_unique: u64,
    /// Members that requested the web version.
    #[serde(deserialize_with = "u64_or_string")]
    pub webversion_unique: u64,
    /// Accepted emails as a fraction of sent emails.
    #[serde(deserialize_with = "f64_or_string")]
    pub accepted_ratio: f64,
    /// Opened emails as a fraction of accepted emails.
    #[serde(deserialize_with = "f64_or_string")]
    pub opened_ratio: f64,
    /// Clicked emails as a fraction of accepted emails.
    #[serde(deserialize_with = "f64_or_string")]
    pub clicked_ratio: f64,
}

/// Campaign reports, obtained from [`Client::reports`].
#[derive(Debug, Clone, Copy)]
pub struct Reports<'a> {
    client: &'a Client,
}

impl<'a> Reports<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Fetches the report of a campaign.
    pub async fn get(&self, campaign_id: &CampaignId) -> Result<Report> {
        let response = self
            .client
            .send(
                Method::GET,
                &["report", campaign_id.as_str()],
                &Query::new(),
                Body::None,
            )
            .await?;
        decode(response, "report")
    }

    /// Fetches the reports of all campaigns.
    pub async fn all(&self) -> Result<Vec<Report>> {
        let response = self
            .client
            .send(Method::GET, &["report"], &Query::new(), Body::None)
            .await?;
        decode_list(response, "report")
    }
}
