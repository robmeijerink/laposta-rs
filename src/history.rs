//! Event history of members and campaigns.

use serde::Deserialize;
use serde_json::Value;

use crate::client::Query;
use crate::pagination::{Page, Pagination};
use crate::types::{CampaignId, Cursor, Limit, ListId, MemberId, SortOrder, Timestamp};

/// Filters and paging for a history request.
#[derive(Debug, Clone, Default)]
pub struct HistoryQuery {
    pub(crate) campaign: Option<CampaignId>,
    pub(crate) from: Option<Timestamp>,
    pub(crate) to: Option<Timestamp>,
    pub(crate) limit: Option<Limit>,
    pub(crate) cursor: Option<Cursor>,
    pub(crate) order: Option<SortOrder>,
}

impl HistoryQuery {
    /// Creates a query without filters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Only events of this campaign. Applies to member history only.
    pub fn campaign(mut self, campaign: CampaignId) -> Self {
        self.campaign = Some(campaign);
        self
    }

    /// Only events on or after this moment.
    pub fn from(mut self, date: Timestamp) -> Self {
        self.from = Some(date);
        self
    }

    /// Only events before this moment.
    pub fn to(mut self, date: Timestamp) -> Self {
        self.to = Some(date);
        self
    }

    /// Page size. Defaults to 100 on the API side.
    pub fn limit(mut self, limit: Limit) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Continues after the given cursor, as returned by a previous page.
    pub fn cursor(mut self, cursor: Cursor) -> Self {
        self.cursor = Some(cursor);
        self
    }

    /// Sort direction of `event_date`. The API defaults to descending.
    pub fn order(mut self, order: SortOrder) -> Self {
        self.order = Some(order);
        self
    }

    pub(crate) fn to_query(&self, list_id: Option<&ListId>) -> Query {
        Query::new()
            .with_opt("list_id", list_id)
            .with_opt("campaign_id", self.campaign.as_ref())
            .with_opt("event_date_from", self.from.as_ref())
            .with_opt("event_date_to", self.to.as_ref())
            .with_opt("limit", self.limit)
            .with_opt("cursor", self.cursor.as_ref())
            .with_opt("order", self.order)
    }
}

/// One page of a member or campaign history.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct History {
    /// The events on this page.
    #[serde(rename = "data")]
    pub events: Vec<Event>,
    /// Cursor information.
    pub pagination: Pagination,
    /// The ordering that was applied.
    pub order: Value,
    /// The filters that were applied.
    pub filters: Value,
}

impl History {
    /// The cursor of the next page, if there is one.
    pub fn next_cursor(&self) -> Option<&Cursor> {
        self.pagination
            .next_cursor
            .as_ref()
            .filter(|_| self.pagination.has_more)
    }

    pub(crate) fn into_page(self) -> Page<Event> {
        Page {
            data: self.events,
            pagination: Some(self.pagination),
        }
    }
}

/// Something that happened to a member or in a campaign.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct Event {
    /// Event type, for example `subscribed`, `opened` or `hardbounced`.
    #[serde(rename = "type")]
    pub kind: String,
    /// Account timezone used for the datetime fields.
    pub timezone: String,
    /// When the event happened.
    pub event_date: String,
    /// When the event happened, in ISO 8601 format.
    pub event_date_iso: String,
    /// The campaign involved, if any.
    pub campaign: Option<EventCampaign>,
    /// The list involved, if any.
    pub list: Option<EventList>,
    /// The member involved, if any.
    pub member: Option<EventMember>,
    /// Type-specific details.
    pub details: Value,
}

/// Campaign reference inside an [`Event`].
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct EventCampaign {
    /// Campaign id.
    pub campaign_id: CampaignId,
    /// Campaign name.
    pub name: String,
    /// Whether the campaign is part of an automation.
    pub from_automation: bool,
    /// Whether the campaign was deleted.
    pub deleted: bool,
}

/// List reference inside an [`Event`].
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct EventList {
    /// List id.
    pub list_id: ListId,
    /// List name.
    pub name: String,
    /// Whether the list was deleted.
    pub deleted: bool,
}

/// Member reference inside an [`Event`].
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct EventMember {
    /// Member id.
    pub member_id: MemberId,
    /// Member email address.
    pub email: String,
    /// Whether the member was deleted.
    pub deleted: bool,
}
