//! Async client for the [Laposta](https://laposta.nl) email marketing API.
//!
//! ```no_run
//! use laposta_rs::{Client, Email, ListId, NewMember};
//!
//! # async fn run() -> laposta_rs::Result<()> {
//! let client = Client::new("your-api-key");
//! let list_id = ListId::new("BaImMu3JZA");
//!
//! let member = NewMember::new(Email::parse("maartje@example.net")?, "198.51.100.10".parse().unwrap())
//!     .custom_field("name", "Maartje");
//! let member = client.members().create(&list_id, &member).await?;
//! println!("added {}", member.member_id);
//! # Ok(())
//! # }
//! ```
//!
//! Identifiers, email addresses, timestamps and page sizes are separate types
//! that are checked when you construct them, so a malformed value never
//! reaches the network.
//!
//! # Features
//!
//! - `rustls-tls` (default): TLS through rustls.
//! - `native-tls`: TLS through the platform library.
//! - `webhook` (default): signature verification and payload types for
//!   incoming webhooks.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod client;
mod error;
mod form;
mod serde_util;

pub mod campaigns;
pub mod fields;
pub mod history;
pub mod lists;
pub mod members;
pub mod pagination;
pub mod reports;
pub mod segments;
pub mod types;
pub mod webhooks;

pub use client::{Client, ClientBuilder};
pub use error::{ApiError, Error, ErrorCode, ErrorType, Result};
pub use pagination::{Page, Pagination};
pub use types::{
    AccountId, CampaignId, Cursor, Email, FieldId, Limit, ListId, MemberId, MemberRef, SegmentId,
    SortOrder, Timestamp, WebhookId,
};

pub use campaigns::{Audience, Campaign, CampaignContent, NewCampaign, Sender, UpdateCampaign};
pub use fields::{Field, NewField, NewFieldKind, UpdateField};
pub use history::{Event, History, HistoryQuery};
pub use lists::{List, NewList, SyncAction, SyncMember, SyncMembers, UpdateList};
pub use members::{
    Member, MemberFilterState, MemberQuery, MemberState, NewMember, UpdateMember,
    WritableMemberState,
};
pub use reports::Report;
pub use segments::{NewSegment, Segment, UpdateSegment};
pub use webhooks::{NewWebhook, UpdateWebhook, Webhook, WebhookEvent};
