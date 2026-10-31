//! Cursor based pagination.

use std::future::Future;

use futures_util::stream::{self, BoxStream};
use futures_util::{StreamExt, TryStreamExt};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::client::decode;
use crate::error::{Error, Result};
use crate::types::Cursor;

/// Cursor information of a paginated response.
#[derive(Debug, Clone, Default, Deserialize)]
#[non_exhaustive]
#[serde(default)]
pub struct Pagination {
    /// Page size used for the request.
    pub limit: u32,
    /// Whether more results follow this page.
    pub has_more: bool,
    /// Cursor to pass to the next request.
    pub next_cursor: Option<Cursor>,
}

/// One page of results.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Page<T> {
    /// Items on this page.
    pub data: Vec<T>,
    /// Cursor information. `None` when the API returned everything at once.
    pub pagination: Option<Pagination>,
}

impl<T> Page<T> {
    /// The cursor of the next page, if there is one.
    pub fn next_cursor(&self) -> Option<&Cursor> {
        let pagination = self.pagination.as_ref()?;
        pagination
            .next_cursor
            .as_ref()
            .filter(|_| pagination.has_more)
    }
}

enum State {
    Start,
    Next(Cursor),
    Done,
}

pub(crate) fn items<T, F, Fut>(fetch: F) -> BoxStream<'static, Result<T>>
where
    T: Send + 'static,
    F: Fn(Option<Cursor>) -> Fut + Send + 'static,
    Fut: Future<Output = Result<Page<T>>> + Send + 'static,
{
    stream::try_unfold(State::Start, move |state| {
        let current = match &state {
            State::Next(cursor) => Some(cursor.clone()),
            _ => None,
        };
        let pending = match state {
            State::Done => None,
            State::Start => Some(fetch(None)),
            State::Next(cursor) => Some(fetch(Some(cursor))),
        };
        async move {
            let Some(pending) = pending else {
                return Ok::<_, Error>(None);
            };
            let page = pending.await?;
            let state = match page.next_cursor() {
                Some(cursor) if Some(cursor) == current.as_ref() => {
                    return Err(Error::Decode {
                        source: serde::de::Error::custom("pagination cursor did not advance"),
                        body: String::new(),
                    });
                }
                Some(cursor) => State::Next(cursor.clone()),
                None => State::Done,
            };
            let items: Vec<Result<T>> = page.data.into_iter().map(Ok).collect();
            Ok(Some((stream::iter(items), state)))
        }
    })
    .try_flatten()
    .boxed()
}

pub(crate) fn decode_page<T: DeserializeOwned>(mut value: Value, key: &str) -> Result<Page<T>> {
    let pagination = value
        .get_mut("pagination")
        .map(Value::take)
        .filter(|p| !p.is_null())
        .map(serde_json::from_value)
        .transpose()
        .map_err(|source| Error::Decode {
            source,
            body: value.to_string().chars().take(500).collect(),
        })?;
    let Some(Value::Array(entries)) = value.get_mut("data").map(Value::take) else {
        return Err(Error::Decode {
            source: serde::de::Error::custom("missing `data` in response"),
            body: value.to_string().chars().take(500).collect(),
        });
    };
    let data = entries
        .into_iter()
        .map(|entry| decode(entry, key))
        .collect::<Result<Vec<T>>>()?;
    Ok(Page { data, pagination })
}
