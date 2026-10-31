use serde::de::Error as _;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

#[derive(Deserialize)]
#[serde(untagged)]
enum Loose {
    Unsigned(u64),
    Float(f64),
    Text(String),
    Null(()),
}

pub(crate) fn u64_or_string<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    Ok(match Loose::deserialize(d)? {
        Loose::Unsigned(n) => n,
        Loose::Float(f) => f as u64,
        Loose::Text(s) => s.trim().parse().map_err(D::Error::custom)?,
        Loose::Null(()) => 0,
    })
}

pub(crate) fn f64_or_string<'de, D: Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
    Ok(match Loose::deserialize(d)? {
        Loose::Unsigned(n) => n as f64,
        Loose::Float(f) => f,
        Loose::Text(s) => s.trim().parse().map_err(D::Error::custom)?,
        Loose::Null(()) => 0.0,
    })
}

pub(crate) fn string_or_number<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(match Loose::deserialize(d)? {
        Loose::Unsigned(n) => n.to_string(),
        Loose::Float(f) => f.to_string(),
        Loose::Text(s) => s,
        Loose::Null(()) => String::new(),
    })
}

/// PHP serialises an empty associative array as `[]`, so accept that where a map is expected.
pub(crate) fn map_or_empty_array<'de, D, M>(d: D) -> Result<M, D::Error>
where
    D: Deserializer<'de>,
    M: serde::de::DeserializeOwned + Default,
{
    match Value::deserialize(d)? {
        Value::Array(items) if items.is_empty() => Ok(M::default()),
        value => M::deserialize(value).map_err(D::Error::custom),
    }
}
