//! Deserializers that never fail the whole line: a field with an unexpected type becomes `None`/zero.
//! Transcripts are not an official format, so one odd field must not drop an entire event.

use std::fmt;

use serde::Deserializer;
use serde::de::{self, IgnoredAny, MapAccess, SeqAccess, Visitor};

/// Shared arms for visitors that only care about one JSON kind and fall back to `$fallback` otherwise.
macro_rules! ignore_non_matching_values {
    ($fallback:expr) => {
        fn visit_u64<E: de::Error>(self, _value: u64) -> Result<Self::Value, E> {
            Ok($fallback)
        }

        fn visit_i64<E: de::Error>(self, _value: i64) -> Result<Self::Value, E> {
            Ok($fallback)
        }

        fn visit_f64<E: de::Error>(self, _value: f64) -> Result<Self::Value, E> {
            Ok($fallback)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok($fallback)
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok($fallback)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
            while sequence.next_element::<IgnoredAny>()?.is_some() {}
            Ok($fallback)
        }

        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
            Ok($fallback)
        }
    };
}

pub fn string<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    deserializer.deserialize_any(StringVisitor)
}

pub fn flag<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    deserializer.deserialize_any(FlagVisitor)
}

pub fn count<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    deserializer.deserialize_any(CountVisitor)
}

struct StringVisitor;

impl<'de> Visitor<'de> for StringVisitor {
    type Value = Option<String>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Some(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(Some(value))
    }

    fn visit_bool<E: de::Error>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(None)
    }

    ignore_non_matching_values!(None);
}

struct FlagVisitor;

impl<'de> Visitor<'de> for FlagVisitor {
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
        Ok(value)
    }

    fn visit_str<E: de::Error>(self, _value: &str) -> Result<Self::Value, E> {
        Ok(false)
    }

    ignore_non_matching_values!(false);
}

struct CountVisitor;

impl<'de> Visitor<'de> for CountVisitor {
    type Value = u64;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        Ok(value)
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        Ok(u64::try_from(value).unwrap_or(0))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        Ok(if value.is_finite() && value > 0.0 { value as u64 } else { 0 })
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(value.parse().unwrap_or(0))
    }

    fn visit_bool<E: de::Error>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(0)
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(0)
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(0)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        while sequence.next_element::<IgnoredAny>()?.is_some() {}
        Ok(0)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(0)
    }
}
