//! Lists and bulk member synchronisation.

use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::client::{Body, Client, Query, decode, decode_list, decode_root, to_value};
use crate::error::{Error, ErrorCode, ErrorType, Result};
use crate::members::{Member, WritableMemberState};
use crate::serde_util::u64_or_string;
use crate::types::{Email, ListId, MemberRef};

/// Whether a list is in use or has been deleted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ListState {
    /// The list is active.
    Active,
    /// The list was deleted.
    Deleted,
    /// A state this crate does not know about.
    #[default]
    #[serde(other)]
    Unknown,
}

/// A list of members.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct List {
    /// List id.
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
    pub state: ListState,
    /// Name of the list.
    pub name: String,
    /// Whether the list is locked against changes in the application.
    pub locked: bool,
    /// Free-form remarks.
    pub remarks: Option<String>,
    /// Address notified when someone subscribes.
    pub subscribe_notification_email: Option<String>,
    /// Address notified when someone unsubscribes.
    pub unsubscribe_notification_email: Option<String>,
    /// Number of members per state. Absent on some responses.
    pub members: Option<MemberCounts>,
}

/// Number of members per state.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct MemberCounts {
    /// Active members.
    pub active: u64,
    /// Unsubscribed members.
    pub unsubscribed: u64,
    /// Members that were cleaned because of bounces or complaints.
    pub cleaned: u64,
}

/// Parameters for creating a list.
#[derive(Debug, Clone, Serialize)]
pub struct NewList {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    locked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    remarks: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subscribe_notification_email: Option<Email>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unsubscribe_notification_email: Option<Email>,
}

impl NewList {
    /// Starts a new list with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            locked: None,
            remarks: None,
            subscribe_notification_email: None,
            unsubscribe_notification_email: None,
        }
    }

    /// Locks the list against changes in the application.
    pub fn locked(mut self, locked: bool) -> Self {
        self.locked = Some(locked);
        self
    }

    /// Sets free-form remarks.
    pub fn remarks(mut self, remarks: impl Into<String>) -> Self {
        self.remarks = Some(remarks.into());
        self
    }

    /// Sets the address that is notified of new subscriptions.
    pub fn subscribe_notification_email(mut self, email: Email) -> Self {
        self.subscribe_notification_email = Some(email);
        self
    }

    /// Sets the address that is notified of unsubscriptions.
    pub fn unsubscribe_notification_email(mut self, email: Email) -> Self {
        self.unsubscribe_notification_email = Some(email);
        self
    }
}

/// Parameters for updating a list. Only fields that are set are changed.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateList {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    locked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    remarks: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subscribe_notification_email: Option<Email>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unsubscribe_notification_email: Option<Email>,
}

impl UpdateList {
    /// Creates an update that changes nothing yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Renames the list.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Locks or unlocks the list.
    pub fn locked(mut self, locked: bool) -> Self {
        self.locked = Some(locked);
        self
    }

    /// Replaces the remarks.
    pub fn remarks(mut self, remarks: impl Into<String>) -> Self {
        self.remarks = Some(remarks.into());
        self
    }

    /// Replaces the subscription notification address.
    pub fn subscribe_notification_email(mut self, email: Email) -> Self {
        self.subscribe_notification_email = Some(email);
        self
    }

    /// Replaces the unsubscription notification address.
    pub fn unsubscribe_notification_email(mut self, email: Email) -> Self {
        self.unsubscribe_notification_email = Some(email);
        self
    }
}

/// Action performed by a bulk member sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SyncAction {
    /// Add members that are not in the list yet.
    Add,
    /// Update members that are already in the list.
    Update,
    /// Unsubscribe every member that is not part of the request.
    UnsubscribeExcluded,
}

/// A member in a bulk sync request.
#[derive(Debug, Clone, Serialize)]
pub struct SyncMember {
    #[serde(skip_serializing_if = "Option::is_none")]
    member_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<Email>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<WritableMemberState>,
    #[serde(skip_serializing_if = "serde_json::Map::is_empty")]
    custom_fields: serde_json::Map<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_url: Option<url::Url>,
}

impl SyncMember {
    /// A member identified by email address, to be added or updated.
    pub fn new(email: Email) -> Self {
        Self {
            member_id: None,
            email: Some(email),
            state: None,
            custom_fields: serde_json::Map::new(),
            source_url: None,
        }
    }

    /// An existing member, identified by id or current email address.
    pub fn existing(member: impl Into<MemberRef>) -> Self {
        Self {
            member_id: Some(member.into().raw().to_owned()),
            email: None,
            state: None,
            custom_fields: serde_json::Map::new(),
            source_url: None,
        }
    }

    /// Sets the email address (for an existing member, changes it).
    pub fn email(mut self, email: Email) -> Self {
        self.email = Some(email);
        self
    }

    /// Sets the member state, for example to unsubscribe the member.
    pub fn state(mut self, state: WritableMemberState) -> Self {
        self.state = Some(state);
        self
    }

    /// Sets a custom field value.
    pub fn custom_field(mut self, name: impl Into<String>, value: impl Into<Value>) -> Self {
        self.custom_fields.insert(name.into(), value.into());
        self
    }

    /// Sets the URL the member signed up on.
    pub fn source_url(mut self, url: url::Url) -> Self {
        self.source_url = Some(url);
        self
    }
}

/// A bulk member sync request. Always carries at least one action.
#[derive(Debug, Clone, Serialize)]
pub struct SyncMembers {
    actions: Vec<SyncAction>,
    members: Vec<SyncMember>,
}

impl SyncMembers {
    /// Starts a sync with its first action.
    pub fn new(action: SyncAction) -> Self {
        Self {
            actions: vec![action],
            members: Vec::new(),
        }
    }

    /// Adds another action.
    pub fn action(mut self, action: SyncAction) -> Self {
        if !self.actions.contains(&action) {
            self.actions.push(action);
        }
        self
    }

    /// Adds a member.
    pub fn member(mut self, member: SyncMember) -> Self {
        self.members.push(member);
        self
    }

    /// Adds several members.
    pub fn members(mut self, members: impl IntoIterator<Item = SyncMember>) -> Self {
        self.members.extend(members);
        self
    }
}

/// Outcome of a bulk member sync.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct SyncResult {
    /// Totals.
    pub report: SyncReport,
    /// Members that could not be processed.
    pub errors: Vec<SyncFailure>,
    /// Members that were left untouched.
    pub skipped: Vec<SyncEntry>,
    /// Members that were updated.
    pub edited: Vec<SyncEntry>,
    /// Members that were added.
    pub added: Vec<SyncEntry>,
}

/// Totals of a bulk member sync.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct SyncReport {
    /// Number of members in the request.
    #[serde(deserialize_with = "u64_or_string")]
    pub provided_count: u64,
    /// Number of members that failed.
    #[serde(deserialize_with = "u64_or_string")]
    pub errors_count: u64,
    /// Number of members that were skipped.
    #[serde(deserialize_with = "u64_or_string")]
    pub skipped_count: u64,
    /// Number of members that were updated.
    #[serde(deserialize_with = "u64_or_string")]
    pub edited_count: u64,
    /// Number of members that were added.
    #[serde(deserialize_with = "u64_or_string")]
    pub added_count: u64,
    /// Number of members unsubscribed by `UnsubscribeExcluded`.
    #[serde(deserialize_with = "u64_or_string")]
    pub unsubscribed_count: u64,
}

/// A processed member of a bulk sync.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct SyncEntry {
    /// The data as submitted.
    pub provided_data: Value,
    /// The member as currently stored, when known.
    pub member: Option<Member>,
}

/// A member of a bulk sync that failed.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct SyncFailure {
    /// The data as submitted.
    pub provided_data: Value,
    /// Why it failed.
    pub error: SyncError,
}

/// Error detail for a failed member in a bulk sync.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct SyncError {
    /// Human readable message.
    pub message: String,
    /// Error code.
    pub code: Option<ErrorCode>,
    /// Offending parameter.
    pub parameter: Option<String>,
    /// Id of the offending object.
    pub id: Option<String>,
    /// Error category.
    #[serde(rename = "type")]
    pub kind: Option<ErrorType>,
}

/// Operations on lists, obtained from [`Client::lists`].
#[derive(Debug, Clone, Copy)]
pub struct Lists<'a> {
    client: &'a Client,
}

impl<'a> Lists<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Creates a list.
    pub async fn create(&self, list: &NewList) -> Result<List> {
        let body = Body::Form(to_value(list)?);
        let response = self
            .client
            .send(Method::POST, &["list"], &Query::new(), body)
            .await?;
        decode(response, "list")
    }

    /// Fetches a list.
    pub async fn get(&self, list_id: &ListId) -> Result<List> {
        let response = self
            .client
            .send(
                Method::GET,
                &["list", list_id.as_str()],
                &Query::new(),
                Body::None,
            )
            .await?;
        decode(response, "list")
    }

    /// Fetches all lists.
    pub async fn all(&self) -> Result<Vec<List>> {
        let response = self
            .client
            .send(Method::GET, &["list"], &Query::new(), Body::None)
            .await?;
        decode_list(response, "list")
    }

    /// Updates a list.
    pub async fn update(&self, list_id: &ListId, update: &UpdateList) -> Result<List> {
        let body = Body::Form(to_value(update)?);
        let response = self
            .client
            .send(
                Method::POST,
                &["list", list_id.as_str()],
                &Query::new(),
                body,
            )
            .await?;
        decode(response, "list")
    }

    /// Permanently deletes a list.
    pub async fn delete(&self, list_id: &ListId) -> Result<List> {
        let response = self
            .client
            .send(
                Method::DELETE,
                &["list", list_id.as_str()],
                &Query::new(),
                Body::None,
            )
            .await?;
        decode(response, "list")
    }

    /// Permanently removes all active members from a list.
    pub async fn purge_members(&self, list_id: &ListId) -> Result<List> {
        let response = self
            .client
            .send(
                Method::DELETE,
                &["list", list_id.as_str(), "members"],
                &Query::new(),
                Body::None,
            )
            .await?;
        decode(response, "list")
    }

    /// Adds, updates and/or unsubscribes many members in one request.
    ///
    /// Not available on free accounts.
    pub async fn sync_members(&self, list_id: &ListId, sync: &SyncMembers) -> Result<SyncResult> {
        if sync.members.is_empty() || sync.members.len() > 500_000 {
            return Err(Error::InvalidInput(
                "a sync needs between 1 and 500000 members".into(),
            ));
        }
        for member in &sync.members {
            if member.member_id.is_none() && member.email.is_none() {
                return Err(Error::InvalidInput(
                    "a sync member needs an id or an email".into(),
                ));
            }
        }
        let body = Body::Json(to_value(sync)?);
        let response = self
            .client
            .send(
                Method::POST,
                &["list", list_id.as_str(), "members"],
                &Query::new(),
                body,
            )
            .await?;
        decode_root(response)
    }
}
