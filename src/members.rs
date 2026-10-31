//! Members (subscribers) of a list.

use std::fmt;
use std::net::IpAddr;

use futures_util::stream::BoxStream;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::client::{Body, Client, Query, decode, decode_root, to_value, with_field};
use crate::error::{Error, Result};
use crate::history::{Event, History, HistoryQuery};
use crate::pagination::{self, Page, decode_page};
use crate::serde_util::map_or_empty_array;
use crate::types::{Cursor, Email, Limit, ListId, MemberId, MemberRef, SortOrder};

/// State of a member.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MemberState {
    /// Subscribed and receiving mail.
    Active,
    /// Unsubscribed.
    Unsubscribed,
    /// Waiting for double opt-in confirmation.
    Unconfirmed,
    /// Removed after bounces or complaints.
    Cleaned,
    /// Deleted.
    Deleted,
    /// A state this crate does not know about.
    #[default]
    #[serde(other)]
    Unknown,
}

impl fmt::Display for MemberState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Active => "active",
            Self::Unsubscribed => "unsubscribed",
            Self::Unconfirmed => "unconfirmed",
            Self::Cleaned => "cleaned",
            Self::Deleted => "deleted",
            Self::Unknown => "unknown",
        })
    }
}

/// A state members can be filtered on when listing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MemberFilterState {
    /// Subscribed and receiving mail.
    Active,
    /// Unsubscribed.
    Unsubscribed,
    /// Removed after bounces or complaints.
    Cleaned,
}

impl fmt::Display for MemberFilterState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Active => "active",
            Self::Unsubscribed => "unsubscribed",
            Self::Cleaned => "cleaned",
        })
    }
}

/// A state a member can be set to through the API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WritableMemberState {
    /// Subscribed and receiving mail.
    Active,
    /// Unsubscribed.
    Unsubscribed,
}

/// A member (subscriber) of a list.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct Member {
    /// Member id.
    pub member_id: MemberId,
    /// The list the member belongs to.
    pub list_id: ListId,
    /// Email address.
    pub email: String,
    /// Current state.
    pub state: MemberState,
    /// Account timezone used for the datetime fields.
    pub timezone: String,
    /// Sign-up time.
    pub signup_date: String,
    /// Sign-up time in ISO 8601 format.
    pub signup_date_iso: String,
    /// Time of the last modification.
    pub modified: Option<String>,
    /// Time of the last modification in ISO 8601 format.
    pub modified_iso: Option<String>,
    /// Time of the double opt-in confirmation.
    pub confirm_date: Option<String>,
    /// Time of the double opt-in confirmation in ISO 8601 format.
    pub confirm_date_iso: Option<String>,
    /// IP address the member signed up from.
    pub ip: Option<String>,
    /// URL the member signed up on.
    pub source_url: Option<String>,
    /// Values of the list's fields, keyed by `custom_name`.
    /// Multiple-choice fields and labels are arrays.
    #[serde(deserialize_with = "map_or_empty_array")]
    pub custom_fields: Map<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize)]
struct MemberOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    upsert: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suppress_reactivation: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suppress_email_notification: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ignore_doubleoptin: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    spam_check: Option<bool>,
}

impl MemberOptions {
    fn is_empty(&self) -> bool {
        self.upsert.is_none()
            && self.suppress_reactivation.is_none()
            && self.suppress_email_notification.is_none()
            && self.ignore_doubleoptin.is_none()
            && self.spam_check.is_none()
    }
}

/// Parameters for adding a member.
#[derive(Debug, Clone, Serialize)]
pub struct NewMember {
    email: Email,
    ip: IpAddr,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_url: Option<url::Url>,
    #[serde(skip_serializing_if = "Map::is_empty")]
    custom_fields: Map<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_agent: Option<String>,
    #[serde(skip_serializing_if = "MemberOptions::is_empty")]
    options: MemberOptions,
}

impl NewMember {
    /// Starts a new member. `ip` is the address the subscriber signed up from.
    pub fn new(email: Email, ip: IpAddr) -> Self {
        Self {
            email,
            ip,
            source_url: None,
            custom_fields: Map::new(),
            user_agent: None,
            options: MemberOptions::default(),
        }
    }

    /// Sets the URL the member signed up on.
    pub fn source_url(mut self, url: url::Url) -> Self {
        self.source_url = Some(url);
        self
    }

    /// Sets a custom field value. Mandatory fields of the list must be supplied.
    pub fn custom_field(mut self, name: impl Into<String>, value: impl Into<Value>) -> Self {
        self.custom_fields.insert(name.into(), value.into());
        self
    }

    /// Sets the subscriber's user agent. Only used by the spam check.
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// Updates the member if it already exists and is active.
    /// Unsubscribed members are reactivated unless reactivation is suppressed.
    pub fn upsert(mut self, upsert: bool) -> Self {
        self.options.upsert = Some(upsert);
        self
    }

    /// Prevents `upsert` from reactivating unsubscribed members.
    pub fn suppress_reactivation(mut self, suppress: bool) -> Self {
        self.options.suppress_reactivation = Some(suppress);
        self
    }

    /// Prevents the subscription notification email.
    pub fn suppress_email_notification(mut self, suppress: bool) -> Self {
        self.options.suppress_email_notification = Some(suppress);
        self
    }

    /// Skips the double opt-in confirmation.
    pub fn ignore_doubleoptin(mut self, ignore: bool) -> Self {
        self.options.ignore_doubleoptin = Some(ignore);
        self
    }

    /// Runs the spam check; rejected sign-ups fail with error code 211.
    pub fn spam_check(mut self, check: bool) -> Self {
        self.options.spam_check = Some(check);
        self
    }
}

/// Parameters for updating a member. Only fields that are set are changed.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateMember {
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<Email>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<WritableMemberState>,
    #[serde(skip_serializing_if = "Map::is_empty")]
    custom_fields: Map<String, Value>,
}

impl UpdateMember {
    /// Creates an update that changes nothing yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Changes the email address.
    pub fn email(mut self, email: Email) -> Self {
        self.email = Some(email);
        self
    }

    /// Changes the member's state.
    pub fn state(mut self, state: WritableMemberState) -> Self {
        self.state = Some(state);
        self
    }

    /// Sets a custom field value. An empty array clears a multiple-choice or labels field.
    pub fn custom_field(mut self, name: impl Into<String>, value: impl Into<Value>) -> Self {
        self.custom_fields.insert(name.into(), value.into());
        self
    }
}

/// Sort key for listing members.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MemberOrder {
    /// Sort by email address.
    Email,
    /// Sort by sign-up date.
    SignupDate,
    /// Sort by a text field, identified by its tag.
    Field(String),
}

impl MemberOrder {
    fn as_str(&self) -> &str {
        match self {
            Self::Email => "email",
            Self::SignupDate => "signup_date",
            Self::Field(tag) => tag,
        }
    }
}

/// Filters, sorting and paging for listing members.
///
/// Without a [`limit`](Self::limit) or [`cursor`](Self::cursor) the API
/// returns every matching member in one response.
#[derive(Debug, Clone, Default)]
pub struct MemberQuery {
    state: Option<MemberFilterState>,
    limit: Option<Limit>,
    cursor: Option<Cursor>,
    order_by: Vec<MemberOrder>,
    order: Option<SortOrder>,
    search: Option<String>,
    label: Option<String>,
}

impl MemberQuery {
    /// Creates a query without filters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Only members in this state.
    pub fn state(mut self, state: MemberFilterState) -> Self {
        self.state = Some(state);
        self
    }

    /// Page size. Enables paging.
    pub fn limit(mut self, limit: Limit) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Continues after the given cursor. Enables paging.
    pub fn cursor(mut self, cursor: Cursor) -> Self {
        self.cursor = Some(cursor);
        self
    }

    /// Adds a sort key.
    pub fn order_by(mut self, key: MemberOrder) -> Self {
        self.order_by.push(key);
        self
    }

    /// Sort direction. The API defaults to ascending.
    pub fn order(mut self, order: SortOrder) -> Self {
        self.order = Some(order);
        self
    }

    /// Searches email addresses and text, numeric and select fields (at most 255 characters).
    pub fn search(mut self, search: impl Into<String>) -> Self {
        self.search = Some(search.into());
        self
    }

    /// Only members with this label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    fn to_query(&self, list_id: &ListId) -> Result<Query> {
        if self
            .search
            .as_ref()
            .is_some_and(|s| s.chars().count() > 255)
        {
            return Err(Error::InvalidInput(
                "search is limited to 255 characters".into(),
            ));
        }
        let order_by = (!self.order_by.is_empty()).then(|| {
            self.order_by
                .iter()
                .map(MemberOrder::as_str)
                .collect::<Vec<_>>()
                .join(",")
        });
        Ok(Query::new()
            .with("list_id", list_id)
            .with_opt("state", self.state)
            .with_opt("limit", self.limit)
            .with_opt("cursor", self.cursor.as_ref())
            .with_opt("order_by", order_by)
            .with_opt("order", self.order)
            .with_opt("search", self.search.as_ref())
            .with_opt("label", self.label.as_ref()))
    }
}

/// Operations on members, obtained from [`Client::members`].
#[derive(Debug, Clone, Copy)]
pub struct Members<'a> {
    client: &'a Client,
}

impl<'a> Members<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Adds a member to a list.
    pub async fn create(&self, list_id: &ListId, member: &NewMember) -> Result<Member> {
        let body = Body::Form(with_field(to_value(member)?, "list_id", list_id.as_str()));
        let response = self
            .client
            .send(Method::POST, &["member"], &Query::new(), body)
            .await?;
        decode(response, "member")
    }

    /// Fetches a member by id or email address.
    pub async fn get(&self, list_id: &ListId, member: impl Into<MemberRef>) -> Result<Member> {
        let segment = member.into().path_segment();
        let query = Query::new().with("list_id", list_id);
        let response = self
            .client
            .send(Method::GET, &["member", &segment], &query, Body::None)
            .await?;
        decode(response, "member")
    }

    /// Fetches one page of members, or all of them when the query does not enable paging.
    pub async fn list(&self, list_id: &ListId, query: &MemberQuery) -> Result<Page<Member>> {
        let query = query.to_query(list_id)?;
        let response = self
            .client
            .send(Method::GET, &["member"], &query, Body::None)
            .await?;
        decode_page(response, "member")
    }

    /// Streams all matching members, fetching pages of up to 1000 as needed.
    pub fn stream(
        &self,
        list_id: &ListId,
        query: MemberQuery,
    ) -> BoxStream<'static, Result<Member>> {
        let client = self.client.clone();
        let list_id = list_id.clone();
        pagination::items(move |cursor| {
            let client = client.clone();
            let list_id = list_id.clone();
            let mut query = query.clone();
            query.cursor = cursor;
            query.limit.get_or_insert_with(Limit::default);
            async move { Members::new(&client).list(&list_id, &query).await }
        })
    }

    /// Updates a member.
    pub async fn update(
        &self,
        list_id: &ListId,
        member: impl Into<MemberRef>,
        update: &UpdateMember,
    ) -> Result<Member> {
        let segment = member.into().path_segment();
        let body = Body::Form(with_field(to_value(update)?, "list_id", list_id.as_str()));
        let response = self
            .client
            .send(Method::POST, &["member", &segment], &Query::new(), body)
            .await?;
        decode(response, "member")
    }

    /// Permanently deletes a member.
    pub async fn delete(&self, list_id: &ListId, member: impl Into<MemberRef>) -> Result<Member> {
        let segment = member.into().path_segment();
        let query = Query::new().with("list_id", list_id);
        let response = self
            .client
            .send(Method::DELETE, &["member", &segment], &query, Body::None)
            .await?;
        decode(response, "member")
    }

    /// Fetches one page of a member's history.
    pub async fn history(
        &self,
        list_id: &ListId,
        member: impl Into<MemberRef>,
        query: &HistoryQuery,
    ) -> Result<History> {
        let segment = member.into().path_segment();
        let response = self
            .client
            .send(
                Method::GET,
                &["member", &segment, "history"],
                &query.to_query(Some(list_id)),
                Body::None,
            )
            .await?;
        decode_root(response)
    }

    /// Streams a member's complete history.
    pub fn history_stream(
        &self,
        list_id: &ListId,
        member: impl Into<MemberRef>,
        query: HistoryQuery,
    ) -> BoxStream<'static, Result<Event>> {
        let client = self.client.clone();
        let list_id = list_id.clone();
        let member = member.into();
        pagination::items(move |cursor| {
            let client = client.clone();
            let list_id = list_id.clone();
            let member = member.clone();
            let mut query = query.clone();
            query.cursor = cursor;
            query.limit.get_or_insert_with(Limit::default);
            async move {
                let history = Members::new(&client)
                    .history(&list_id, member, &query)
                    .await?;
                Ok(history.into_page())
            }
        })
    }
}
