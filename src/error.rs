use std::fmt;
use std::time::Duration;

use serde::Deserialize;

/// Result type used throughout the crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors returned by the client.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The API answered with a non-success status.
    #[error(transparent)]
    Api(#[from] ApiError),
    /// The request could not be sent or the response could not be read.
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    /// The response body did not have the expected shape.
    #[error("failed to decode response: {source}")]
    Decode {
        /// Underlying deserialization error.
        source: serde_json::Error,
        /// The (possibly truncated) response body. It may contain personal data, so
        /// think twice before logging it.
        body: String,
    },
    /// The request was rejected before it was sent.
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

impl Error {
    /// Returns the API error, if this is one.
    pub fn as_api(&self) -> Option<&ApiError> {
        match self {
            Error::Api(e) => Some(e),
            _ => None,
        }
    }

    /// Whether the API reported that the requested object or URL does not exist.
    pub fn is_not_found(&self) -> bool {
        self.as_api().is_some_and(ApiError::is_not_found)
    }

    /// Whether the request was rejected because of rate limiting.
    pub fn is_rate_limited(&self) -> bool {
        self.as_api().is_some_and(ApiError::is_rate_limited)
    }

    /// Whether the API key was missing or invalid.
    pub fn is_unauthorized(&self) -> bool {
        self.as_api().is_some_and(ApiError::is_unauthorized)
    }
}

/// An error response from the Laposta API.
#[derive(Debug, Clone)]
pub struct ApiError {
    /// HTTP status code.
    pub status: u16,
    /// Error category reported by the API.
    pub kind: ErrorType,
    /// Human readable message.
    pub message: String,
    /// Numeric error code, when the API supplies one.
    pub code: Option<ErrorCode>,
    /// Name of the offending request parameter, when known.
    pub parameter: Option<String>,
    /// Value of the `Retry-After` header, when present.
    pub retry_after: Option<Duration>,
}

impl ApiError {
    pub(crate) fn from_response(status: u16, body: &[u8], retry_after: Option<Duration>) -> Self {
        #[derive(Deserialize)]
        struct Envelope {
            error: Raw,
        }
        #[derive(Deserialize)]
        struct Raw {
            #[serde(rename = "type")]
            kind: Option<String>,
            message: Option<String>,
            code: Option<u16>,
            parameter: Option<String>,
        }

        match serde_json::from_slice::<Envelope>(body) {
            Ok(Envelope { error }) => Self {
                status,
                kind: error
                    .kind
                    .as_deref()
                    .map_or(ErrorType::Unknown, ErrorType::from_name),
                message: error.message.unwrap_or_default(),
                code: error.code.map(ErrorCode::from),
                parameter: error.parameter,
                retry_after,
            },
            Err(_) => Self {
                status,
                kind: ErrorType::Unknown,
                message: String::from_utf8_lossy(body).chars().take(500).collect(),
                code: None,
                parameter: None,
                retry_after,
            },
        }
    }

    /// Whether the API reported that the requested object or URL does not exist.
    pub fn is_not_found(&self) -> bool {
        self.status == 404 || self.code == Some(ErrorCode::UnknownObject)
    }

    /// Whether the request was rejected because of rate limiting.
    pub fn is_rate_limited(&self) -> bool {
        self.status == 429
    }

    /// Whether the API key was missing or invalid.
    pub fn is_unauthorized(&self) -> bool {
        self.status == 401
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "laposta api error (status {}", self.status)?;
        if let Some(code) = self.code {
            write!(f, ", code {}", u16::from(code))?;
        }
        if let Some(parameter) = &self.parameter {
            write!(f, ", parameter {parameter}")?;
        }
        write!(f, "): {}", self.message)
    }
}

impl std::error::Error for ApiError {}

/// Category of an API error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ErrorType {
    /// The URL or the way the API was invoked is wrong.
    InvalidRequest,
    /// The supplied data is wrong.
    InvalidInput,
    /// Something went wrong on the Laposta side.
    Internal,
    /// Missing or unrecognised type.
    #[serde(other)]
    Unknown,
}

impl ErrorType {
    fn from_name(name: &str) -> Self {
        match name {
            "invalid_request" => Self::InvalidRequest,
            "invalid_input" => Self::InvalidInput,
            "internal" => Self::Internal,
            _ => Self::Unknown,
        }
    }
}

/// Numeric error codes documented by the API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorCode {
    /// 201: a parameter is empty.
    ParameterEmpty,
    /// 202: a parameter has an incorrect syntax.
    InvalidSyntax,
    /// 203: unknown parameter or object.
    UnknownObject,
    /// 204: the object already exists.
    AlreadyExists,
    /// 205: not a number.
    NotANumber,
    /// 206: not a boolean.
    NotABoolean,
    /// 207: not a date.
    NotADate,
    /// 208: not an email address.
    NotAnEmail,
    /// 209: not a URL.
    NotAUrl,
    /// 210: not valid JSON.
    NotJson,
    /// 211: rejected by the spam check.
    SpamRejected,
    /// 212: address blocked because it was cleaned earlier.
    BlockedCleaned,
    /// 213: address blocked because it unsubscribed earlier and its data was removed.
    BlockedUnsubscribed,
    /// 214: the processor agreement has not been signed.
    ProcessorAgreementMissing,
    /// 215: a parameter is too long.
    TooLong,
    /// 999: other error in a parameter.
    Other,
    /// A code this crate does not know about.
    Unrecognized(u16),
}

impl From<u16> for ErrorCode {
    fn from(code: u16) -> Self {
        match code {
            201 => Self::ParameterEmpty,
            202 => Self::InvalidSyntax,
            203 => Self::UnknownObject,
            204 => Self::AlreadyExists,
            205 => Self::NotANumber,
            206 => Self::NotABoolean,
            207 => Self::NotADate,
            208 => Self::NotAnEmail,
            209 => Self::NotAUrl,
            210 => Self::NotJson,
            211 => Self::SpamRejected,
            212 => Self::BlockedCleaned,
            213 => Self::BlockedUnsubscribed,
            214 => Self::ProcessorAgreementMissing,
            215 => Self::TooLong,
            999 => Self::Other,
            other => Self::Unrecognized(other),
        }
    }
}

impl From<ErrorCode> for u16 {
    fn from(code: ErrorCode) -> Self {
        match code {
            ErrorCode::ParameterEmpty => 201,
            ErrorCode::InvalidSyntax => 202,
            ErrorCode::UnknownObject => 203,
            ErrorCode::AlreadyExists => 204,
            ErrorCode::NotANumber => 205,
            ErrorCode::NotABoolean => 206,
            ErrorCode::NotADate => 207,
            ErrorCode::NotAnEmail => 208,
            ErrorCode::NotAUrl => 209,
            ErrorCode::NotJson => 210,
            ErrorCode::SpamRejected => 211,
            ErrorCode::BlockedCleaned => 212,
            ErrorCode::BlockedUnsubscribed => 213,
            ErrorCode::ProcessorAgreementMissing => 214,
            ErrorCode::TooLong => 215,
            ErrorCode::Other => 999,
            ErrorCode::Unrecognized(other) => other,
        }
    }
}

impl<'de> Deserialize<'de> for ErrorCode {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        u16::deserialize(d).map(Self::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_error_envelope() {
        let body = br#"{"error":{"type":"invalid_input","message":"Invalid email","code":208,"parameter":"email"}}"#;
        let err = ApiError::from_response(400, body, None);
        assert_eq!(err.kind, ErrorType::InvalidInput);
        assert_eq!(err.code, Some(ErrorCode::NotAnEmail));
        assert_eq!(err.parameter.as_deref(), Some("email"));
        assert_eq!(err.message, "Invalid email");
        assert_eq!(
            err.to_string(),
            "laposta api error (status 400, code 208, parameter email): Invalid email"
        );
    }

    #[test]
    fn falls_back_to_raw_body() {
        let err = ApiError::from_response(502, b"<html>Bad gateway</html>", None);
        assert_eq!(err.kind, ErrorType::Unknown);
        assert_eq!(err.message, "<html>Bad gateway</html>");
    }

    #[test]
    fn unknown_object_counts_as_not_found() {
        let body = br#"{"error":{"type":"invalid_input","message":"Unknown list","code":203}}"#;
        assert!(ApiError::from_response(400, body, None).is_not_found());
    }

    #[test]
    fn code_round_trips() {
        for n in [201, 208, 214, 215, 999, 777] {
            assert_eq!(u16::from(ErrorCode::from(n)), n);
        }
    }
}
