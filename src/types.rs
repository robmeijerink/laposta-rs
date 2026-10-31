//! Identifiers and validated value types.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::Error;

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Wraps an identifier as returned by the API.
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            /// The identifier as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(id: &str) -> Self {
                Self::new(id)
            }
        }

        impl From<String> for $name {
            fn from(id: String) -> Self {
                Self(id)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
    };
}

id_type!(
    /// Identifier of a list.
    ListId
);
id_type!(
    /// Identifier of a list field.
    FieldId
);
id_type!(
    /// Identifier of a segment.
    SegmentId
);
id_type!(
    /// Identifier of a member.
    MemberId
);
id_type!(
    /// Identifier of a campaign.
    CampaignId
);
id_type!(
    /// Identifier of a webhook.
    WebhookId
);
id_type!(
    /// Identifier of an account.
    AccountId
);

id_type!(
    /// Opaque cursor returned by a paginated endpoint.
    Cursor
);

/// An email address that passed a basic sanity check.
///
/// This catches obvious mistakes before a request is sent. The API remains the
/// authority on what is a valid address.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct Email(String);

impl Email {
    /// Parses an email address.
    pub fn parse(address: impl AsRef<str>) -> Result<Self, ParseEmailError> {
        let address = address.as_ref();
        let invalid = || ParseEmailError(address.to_owned());
        if address.len() > 254 || address.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(invalid());
        }
        let (local, domain) = address.rsplit_once('@').ok_or_else(invalid)?;
        if local.is_empty() || local.len() > 64 || local.contains('@') || domain.is_empty() {
            return Err(invalid());
        }
        if domain.starts_with('.') || domain.ends_with('.') || domain.contains("..") {
            return Err(invalid());
        }
        Ok(Self(address.to_owned()))
    }

    /// The address as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Email {
    type Err = ParseEmailError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for Email {
    type Error = ParseEmailError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl TryFrom<String> for Email {
    type Error = ParseEmailError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for Email {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// The supplied string is not a plausible email address.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a valid email address")]
pub struct ParseEmailError(String);

impl From<ParseEmailError> for Error {
    fn from(e: ParseEmailError) -> Self {
        Error::InvalidInput(e.to_string())
    }
}

/// Identifies a member either by its id or by its email address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemberRef {
    /// The Laposta member id.
    Id(MemberId),
    /// The member's email address.
    Email(Email),
}

impl MemberRef {
    pub(crate) fn raw(&self) -> &str {
        match self {
            Self::Id(id) => id.as_str(),
            Self::Email(email) => email.as_str(),
        }
    }

    pub(crate) fn path_segment(&self) -> String {
        match self {
            Self::Id(id) => id.as_str().to_owned(),
            Self::Email(email) => email.as_str().replace('+', "%2B"),
        }
    }
}

impl From<MemberId> for MemberRef {
    fn from(id: MemberId) -> Self {
        Self::Id(id)
    }
}

impl From<&MemberId> for MemberRef {
    fn from(id: &MemberId) -> Self {
        Self::Id(id.clone())
    }
}

impl From<Email> for MemberRef {
    fn from(email: Email) -> Self {
        Self::Email(email)
    }
}

impl From<&Email> for MemberRef {
    fn from(email: &Email) -> Self {
        Self::Email(email.clone())
    }
}

/// A date and time in one of the two formats the API accepts:
/// `YYYY-MM-DD HH:MM:SS` (account timezone) or RFC 3339 with offset.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct Timestamp(String);

impl Timestamp {
    /// Parses a timestamp.
    pub fn parse(value: impl AsRef<str>) -> Result<Self, ParseTimestampError> {
        let value = value.as_ref();
        if is_timestamp(value) {
            Ok(Self(value.to_owned()))
        } else {
            Err(ParseTimestampError(value.to_owned()))
        }
    }

    /// The timestamp as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_timestamp(value: &str) -> bool {
    let b = value.as_bytes();
    let digits = |range: std::ops::Range<usize>| -> Option<u32> {
        let part = b.get(range)?;
        part.iter()
            .all(u8::is_ascii_digit)
            .then(|| part.iter().fold(0, |acc, d| acc * 10 + u32::from(d - b'0')))
    };
    let separators = b.get(4) == Some(&b'-')
        && b.get(7) == Some(&b'-')
        && b.get(13) == Some(&b':')
        && b.get(16) == Some(&b':');
    let (Some(_), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        digits(0..4),
        digits(5..7),
        digits(8..10),
        digits(11..13),
        digits(14..16),
        digits(17..19),
    ) else {
        return false;
    };
    if !separators || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return false;
    }
    if hour > 23 || minute > 59 || second > 60 {
        return false;
    }
    match (b.get(10), &value[19..]) {
        (Some(b' '), "") => true,
        (Some(b'T'), rest) => {
            let rest = match rest.strip_prefix('.') {
                Some(fraction) => {
                    let end = fraction
                        .find(|c: char| !c.is_ascii_digit())
                        .unwrap_or(fraction.len());
                    if end == 0 {
                        return false;
                    }
                    &fraction[end..]
                }
                None => rest,
            };
            if rest == "Z" {
                return true;
            }
            let Some(offset) = rest.strip_prefix(['+', '-']) else {
                return false;
            };
            let ob = offset.as_bytes();
            ob.len() == 5
                && ob[2] == b':'
                && ob[..2].iter().chain(&ob[3..]).all(u8::is_ascii_digit)
                && offset[..2].parse::<u8>().is_ok_and(|h| h < 24)
                && offset[3..].parse::<u8>().is_ok_and(|m| m < 60)
        }
        _ => false,
    }
}

impl FromStr for Timestamp {
    type Err = ParseTimestampError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The supplied string is not a supported timestamp format.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a timestamp like `2026-05-18 14:30:00` or `2026-05-18T14:30:00+02:00`")]
pub struct ParseTimestampError(String);

impl From<ParseTimestampError> for Error {
    fn from(e: ParseTimestampError) -> Self {
        Error::InvalidInput(e.to_string())
    }
}

/// Page size of a paginated request, between 1 and 1000.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Limit(u16);

impl Limit {
    /// Smallest page size accepted by the API.
    pub const MIN: u16 = 1;
    /// Largest page size accepted by the API.
    pub const MAX: u16 = 1000;

    /// Creates a limit, or `None` when it is outside `1..=1000`.
    pub fn new(limit: u16) -> Option<Self> {
        (Self::MIN..=Self::MAX)
            .contains(&limit)
            .then_some(Self(limit))
    }

    /// The page size.
    pub fn get(self) -> u16 {
        self.0
    }
}

impl Default for Limit {
    fn default() -> Self {
        Self(Self::MAX)
    }
}

impl fmt::Display for Limit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    /// Ascending.
    Asc,
    /// Descending.
    Desc,
}

impl fmt::Display for SortOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plausible_emails() {
        for ok in ["a@b.nl", "first+tag@example.org", "x@localhost"] {
            assert!(Email::parse(ok).is_ok(), "{ok}");
        }
    }

    #[test]
    fn rejects_implausible_emails() {
        for bad in [
            "", "a", "@b.nl", "a@", "a b@c.nl", "a@@b.nl", "a@b..nl", "a@.nl", "a@b.nl.",
        ] {
            assert!(Email::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn email_member_ref_escapes_plus() {
        let member = MemberRef::from(Email::parse("a+b@example.org").unwrap());
        assert_eq!(member.path_segment(), "a%2Bb@example.org");
    }

    #[test]
    fn timestamp_formats() {
        for ok in [
            "2026-05-18 14:30:00",
            "2026-05-18T14:30:00+02:00",
            "2026-05-18T14:30:00Z",
            "2026-05-18T14:30:00.123-05:30",
        ] {
            assert!(Timestamp::parse(ok).is_ok(), "{ok}");
        }
        for bad in [
            "",
            "2026-05-18",
            "2026-13-18 14:30:00",
            "2026-05-18 24:00:00",
            "2026-05-18T14:30:00",
            "2026-05-18 14:30:00Z",
            "2026-05-18T14:30:00+0200",
            "18-05-2026 14:30:00",
        ] {
            assert!(Timestamp::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn limit_is_bounded() {
        assert!(Limit::new(0).is_none());
        assert!(Limit::new(1001).is_none());
        assert_eq!(Limit::new(250).unwrap().get(), 250);
    }
}
