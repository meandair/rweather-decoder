//! Custom datetime module representing dates and times in the UTC standard.

use anyhow::Result;
use chrono::{NaiveTime, NaiveDateTime};
use serde::{de, Serialize, Deserialize, Deserializer, ser::SerializeTuple};

const UTC_DATE_TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%SZ";
const UTC_TIME_FORMAT: &str = "%H:%M:%SZ";

/// UTC datetime wrapper around [chrono]'s [`NaiveDateTime`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtcDateTime(pub NaiveDateTime);

impl Serialize for UtcDateTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.format(UTC_DATE_TIME_FORMAT).to_string())
    }
}

impl<'de> Deserialize<'de> for UtcDateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self(NaiveDateTime::parse_from_str(&String::deserialize(deserializer)?, UTC_DATE_TIME_FORMAT)
            .map_err(de::Error::custom)?))
    }
}

/// UTC day-time wrapper around integer day and [chrono]'s [`NaiveTime`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtcDayTime(pub u32, pub NaiveTime);

impl Serialize for UtcDayTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let time = self.1.format(UTC_TIME_FORMAT).to_string();

        let mut tup = serializer.serialize_tuple(2)?;
        tup.serialize_element(&self.0)?;
        tup.serialize_element(&time)?;
        tup.end()
    }
}

impl<'de> Deserialize<'de> for UtcDayTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (d, time) = <(u32, String)>::deserialize(deserializer)?;
        let nt = NaiveTime::parse_from_str(&time, UTC_TIME_FORMAT)
            .map_err(de::Error::custom)?;

        Ok(Self(d, nt))
    }
}

/// UTC time wrapper around [chrono]'s [`NaiveTime`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtcTime(pub NaiveTime);

impl Serialize for UtcTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.format(UTC_TIME_FORMAT).to_string())
    }
}

impl<'de> Deserialize<'de> for UtcTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self(NaiveTime::parse_from_str(&s, UTC_TIME_FORMAT).map_err(de::Error::custom)?))
    }
}
