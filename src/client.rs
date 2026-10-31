use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use reqwest::Method;
use reqwest::header::{CONTENT_TYPE, RETRY_AFTER, USER_AGENT};
use reqwest::redirect::Policy;
use serde::de::DeserializeOwned;
use serde_json::Value;
use url::Url;

use crate::campaigns::Campaigns;
use crate::error::{ApiError, Error, Result};
use crate::fields::Fields;
use crate::form;
use crate::lists::Lists;
use crate::members::Members;
use crate::reports::Reports;
use crate::segments::Segments;
use crate::webhooks::Webhooks;

const DEFAULT_BASE_URL: &str = "https://api.laposta.nl/v2/";
const DEFAULT_RETRY_DELAY: Duration = Duration::from_secs(1);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(60);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

/// Client for the Laposta API.
///
/// Cloning is cheap; clones share the underlying connection pool.
#[derive(Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

struct Inner {
    http: reqwest::Client,
    api_key: String,
    base_url: Url,
    max_retries: u32,
    timeout: Duration,
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.inner.base_url.as_str())
            .field("max_retries", &self.inner.max_retries)
            .finish_non_exhaustive()
    }
}

impl Client {
    /// Creates a client with default settings.
    ///
    /// # Panics
    ///
    /// Panics if the TLS backend cannot be initialised. Use [`Client::builder`]
    /// to handle that case.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::builder()
            .api_key(api_key)
            .build()
            .expect("failed to initialise the HTTP client")
    }

    /// Returns a builder for customising the client.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// Operations on lists.
    pub fn lists(&self) -> Lists<'_> {
        Lists::new(self)
    }

    /// Operations on list fields.
    pub fn fields(&self) -> Fields<'_> {
        Fields::new(self)
    }

    /// Operations on segments.
    pub fn segments(&self) -> Segments<'_> {
        Segments::new(self)
    }

    /// Operations on members (subscribers).
    pub fn members(&self) -> Members<'_> {
        Members::new(self)
    }

    /// Operations on campaigns.
    pub fn campaigns(&self) -> Campaigns<'_> {
        Campaigns::new(self)
    }

    /// Campaign reports.
    pub fn reports(&self) -> Reports<'_> {
        Reports::new(self)
    }

    /// Operations on webhooks.
    pub fn webhooks(&self) -> Webhooks<'_> {
        Webhooks::new(self)
    }

    pub(crate) async fn send(
        &self,
        method: Method,
        segments: &[&str],
        query: &Query,
        body: Body,
    ) -> Result<Value> {
        let url = self.url(segments, query)?;
        let mut attempt = 0;
        loop {
            let mut request = self
                .inner
                .http
                .request(method.clone(), url.clone())
                .basic_auth(&self.inner.api_key, Some(""))
                .timeout(self.inner.timeout)
                .header(
                    USER_AGENT,
                    concat!("laposta-rs/", env!("CARGO_PKG_VERSION")),
                );
            request = match &body {
                Body::None => request,
                Body::Form(value) => request
                    .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(form::encode(value)),
                Body::Json(value) => request
                    .header(CONTENT_TYPE, "application/json")
                    .body(value.to_string()),
            };

            let response = request.send().await?;
            let status = response.status();
            let retry_after = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.trim().parse::<u64>().ok())
                .map(Duration::from_secs);
            let bytes = response.bytes().await?;

            if status.is_success() {
                if bytes.is_empty() {
                    return Ok(Value::Null);
                }
                return serde_json::from_slice(&bytes).map_err(|source| Error::Decode {
                    source,
                    body: truncate(&String::from_utf8_lossy(&bytes)),
                });
            }

            if status.as_u16() == 429 && attempt < self.inner.max_retries {
                attempt += 1;
                tokio::time::sleep(
                    retry_after
                        .unwrap_or(DEFAULT_RETRY_DELAY)
                        .min(MAX_RETRY_DELAY),
                )
                .await;
                continue;
            }
            return Err(ApiError::from_response(status.as_u16(), &bytes, retry_after).into());
        }
    }

    fn url(&self, segments: &[&str], query: &Query) -> Result<Url> {
        for segment in segments {
            if segment.is_empty()
                || matches!(*segment, "." | "..")
                || segment.chars().any(char::is_control)
            {
                return Err(Error::InvalidInput(format!(
                    "`{}` cannot be used as an identifier",
                    segment.escape_debug()
                )));
            }
        }
        let mut url = self.inner.base_url.clone();
        url.path_segments_mut()
            .expect("base url was validated by the builder")
            .pop_if_empty()
            .extend(segments);
        if !query.0.is_empty() {
            url.query_pairs_mut().extend_pairs(&query.0);
        }
        Ok(url)
    }
}

/// Builder for [`Client`].
#[derive(Default)]
pub struct ClientBuilder {
    api_key: Option<String>,
    base_url: Option<String>,
    http: Option<reqwest::Client>,
    max_retries: u32,
    timeout: Option<Duration>,
}

impl fmt::Debug for ClientBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientBuilder")
            .field("base_url", &self.base_url)
            .field("max_retries", &self.max_retries)
            .finish_non_exhaustive()
    }
}

impl ClientBuilder {
    /// Sets the API key. Required.
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Overrides the API base URL, for example to point at a proxy or a mock server.
    ///
    /// Must use `https`; plain `http` is only accepted for loopback addresses.
    /// Defaults to `https://api.laposta.nl/v2/`.
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Uses an existing [`reqwest::Client`], for custom proxies or TLS settings.
    ///
    /// The client is used as is. In particular, redirects are followed unless
    /// you disable them, which the default client does.
    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.http = Some(client);
        self
    }

    /// Number of times a request that hit the rate limit (HTTP 429) is retried
    /// after waiting for the `Retry-After` duration. Defaults to 0.
    ///
    /// Waiting requires a Tokio runtime with the time driver enabled.
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Time allowed for a single request, including reading the response.
    /// Defaults to 60 seconds. Large bulk syncs may need more.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Builds the client.
    pub fn build(self) -> Result<Client> {
        let api_key = self
            .api_key
            .filter(|k| !k.is_empty())
            .ok_or_else(|| Error::InvalidInput("an API key is required".into()))?;
        let base_url = Url::parse(self.base_url.as_deref().unwrap_or(DEFAULT_BASE_URL))
            .map_err(|e| Error::InvalidInput(format!("invalid base url: {e}")))?;
        if base_url.cannot_be_a_base() {
            return Err(Error::InvalidInput(
                "base url cannot be used as a base".into(),
            ));
        }
        if base_url.scheme() != "https" && !(base_url.scheme() == "http" && is_loopback(&base_url))
        {
            return Err(Error::InvalidInput(
                "base url must use https, or http on a loopback address".into(),
            ));
        }
        let http = match self.http {
            Some(http) => http,
            None => reqwest::Client::builder()
                .connect_timeout(CONNECT_TIMEOUT)
                .redirect(Policy::none())
                .build()?,
        };
        Ok(Client {
            inner: Arc::new(Inner {
                http,
                api_key,
                base_url,
                max_retries: self.max_retries,
                timeout: self.timeout.unwrap_or(DEFAULT_TIMEOUT),
            }),
        })
    }
}

fn is_loopback(url: &Url) -> bool {
    match url.host() {
        Some(url::Host::Domain(domain)) => domain.eq_ignore_ascii_case("localhost"),
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    }
}

pub(crate) enum Body {
    None,
    Form(Value),
    Json(Value),
}

#[derive(Default)]
pub(crate) struct Query(Vec<(String, String)>);

impl Query {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn with(mut self, name: &str, value: impl ToString) -> Self {
        self.0.push((name.to_owned(), value.to_string()));
        self
    }

    pub(crate) fn with_opt(self, name: &str, value: Option<impl ToString>) -> Self {
        match value {
            Some(value) => self.with(name, value),
            None => self,
        }
    }
}

pub(crate) fn to_value(request: &impl serde::Serialize) -> Result<Value> {
    serde_json::to_value(request).map_err(|e| Error::InvalidInput(e.to_string()))
}

pub(crate) fn with_field(mut value: Value, name: &str, field: &str) -> Value {
    if let Value::Object(map) = &mut value {
        map.insert(name.to_owned(), Value::String(field.to_owned()));
    }
    value
}

pub(crate) fn decode_root<T: DeserializeOwned>(value: Value) -> Result<T> {
    let body = value.clone();
    serde_json::from_value(value).map_err(|source| Error::Decode {
        source,
        body: truncate(&body.to_string()),
    })
}

pub(crate) fn decode<T: DeserializeOwned>(mut value: Value, key: &str) -> Result<T> {
    let inner = value.get_mut(key).map(Value::take);
    decode_value(inner, &value, key)
}

pub(crate) fn decode_list<T: DeserializeOwned>(mut value: Value, key: &str) -> Result<Vec<T>> {
    let Some(Value::Array(entries)) = value.get_mut("data").map(Value::take) else {
        return Err(missing(&value, "data"));
    };
    entries
        .into_iter()
        .map(|entry| decode(entry, key))
        .collect()
}

fn decode_value<T: DeserializeOwned>(inner: Option<Value>, outer: &Value, key: &str) -> Result<T> {
    match inner {
        Some(inner) => serde_json::from_value(inner).map_err(|source| Error::Decode {
            source,
            body: truncate(&outer.to_string()),
        }),
        None => Err(missing(outer, key)),
    }
}

fn missing(outer: &Value, key: &str) -> Error {
    Error::Decode {
        source: serde::de::Error::custom(format!("missing `{key}` in response")),
        body: truncate(&outer.to_string()),
    }
}

fn truncate(body: &str) -> String {
    body.chars().take(500).collect()
}
