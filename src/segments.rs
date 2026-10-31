//! Segments of a list.

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::client::{Body, Client, Query, decode, decode_list, to_value, with_field};
use crate::error::Result;
use crate::types::{ListId, SegmentId};

/// A segment of a list.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct Segment {
    /// Segment id.
    pub segment_id: SegmentId,
    /// The list the segment belongs to.
    pub list_id: ListId,
    /// Name of the segment.
    pub name: String,
    /// Definition of the segment, a string in Laposta's own JSON format.
    pub definition: String,
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
}

/// Parameters for creating a segment.
#[derive(Debug, Clone, Serialize)]
pub struct NewSegment {
    name: String,
    definition: String,
}

impl NewSegment {
    /// Creates a segment. The easiest way to get a valid definition is to
    /// build the segment in the application and read it back through the API.
    pub fn new(name: impl Into<String>, definition: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            definition: definition.into(),
        }
    }
}

/// Parameters for updating a segment. Only fields that are set are changed.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateSegment {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    definition: Option<String>,
}

impl UpdateSegment {
    /// Creates an update that changes nothing yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Renames the segment.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Replaces the definition.
    pub fn definition(mut self, definition: impl Into<String>) -> Self {
        self.definition = Some(definition.into());
        self
    }
}

/// Operations on segments, obtained from [`Client::segments`].
#[derive(Debug, Clone, Copy)]
pub struct Segments<'a> {
    client: &'a Client,
}

impl<'a> Segments<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Creates a segment in a list.
    pub async fn create(&self, list_id: &ListId, segment: &NewSegment) -> Result<Segment> {
        let body = Body::Form(with_field(to_value(segment)?, "list_id", list_id.as_str()));
        let response = self
            .client
            .send(Method::POST, &["segment"], &Query::new(), body)
            .await?;
        decode(response, "segment")
    }

    /// Fetches a segment.
    pub async fn get(&self, list_id: &ListId, segment_id: &SegmentId) -> Result<Segment> {
        let query = Query::new().with("list_id", list_id);
        let response = self
            .client
            .send(
                Method::GET,
                &["segment", segment_id.as_str()],
                &query,
                Body::None,
            )
            .await?;
        decode(response, "segment")
    }

    /// Fetches all segments of a list.
    pub async fn all(&self, list_id: &ListId) -> Result<Vec<Segment>> {
        let query = Query::new().with("list_id", list_id);
        let response = self
            .client
            .send(Method::GET, &["segment"], &query, Body::None)
            .await?;
        decode_list(response, "segment")
    }

    /// Updates a segment.
    pub async fn update(
        &self,
        list_id: &ListId,
        segment_id: &SegmentId,
        update: &UpdateSegment,
    ) -> Result<Segment> {
        let body = Body::Form(with_field(to_value(update)?, "list_id", list_id.as_str()));
        let response = self
            .client
            .send(
                Method::POST,
                &["segment", segment_id.as_str()],
                &Query::new(),
                body,
            )
            .await?;
        decode(response, "segment")
    }

    /// Permanently deletes a segment.
    pub async fn delete(&self, list_id: &ListId, segment_id: &SegmentId) -> Result<Segment> {
        let query = Query::new().with("list_id", list_id);
        let response = self
            .client
            .send(
                Method::DELETE,
                &["segment", segment_id.as_str()],
                &query,
                Body::None,
            )
            .await?;
        decode(response, "segment")
    }
}
