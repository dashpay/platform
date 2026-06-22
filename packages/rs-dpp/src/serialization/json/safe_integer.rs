//! Serde `with` modules for bare u64/i64 fields and their Option variants.
//!
//! These are automatically added by the `#[json_safe_fields]` attribute macro.
//! You should not normally need to reference them directly.
//!
//! ## Behavior
//!
//! - **JSON** (`is_human_readable() == true`): values > `MAX_SAFE_INTEGER` (2^53 - 1)
//!   are serialized as strings. Deserialization accepts both numbers and strings.
//! - **platform_value / bincode** (`is_human_readable() == false`): native integer
//!   representation, no transformation.
//!
//! ## Available modules
//!
//! - [`json_safe_u64`] — for `u64` fields
//! - [`json_safe_i64`] — for `i64` fields
//! - [`json_safe_option_u64`] — for `Option<u64>` fields
//! - [`json_safe_option_i64`] — for `Option<i64>` fields

pub(crate) const JS_MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// Serde `with` module for `u64` fields.
pub mod json_safe_u64 {
    use serde::de::{self, Deserializer, Visitor};
    use serde::ser::Serializer;

    use super::JS_MAX_SAFE_INTEGER;

    pub fn serialize<S: Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() && *value > JS_MAX_SAFE_INTEGER {
            serializer.serialize_str(&value.to_string())
        } else {
            serializer.serialize_u64(*value)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_any(U64OrStringVisitor)
        } else {
            serde::Deserialize::deserialize(deserializer)
        }
    }

    struct U64OrStringVisitor;

    impl<'de> Visitor<'de> for U64OrStringVisitor {
        type Value = u64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a u64 or a string containing a u64")
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
            Ok(v)
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
            u64::try_from(v)
                .map_err(|_| de::Error::custom(format!("i64 value {v} out of u64 range")))
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            v.parse::<u64>()
                .map_err(|_| de::Error::custom(format!("invalid u64 string: {v}")))
        }
    }
}

/// Serde `with` module for `u128` fields.
///
/// `u128` is never JS-safe as a bare number once it exceeds
/// `Number.MAX_SAFE_INTEGER`, so the human-readable (JSON) path stringifies
/// large values. The binary / `Value` path keeps the native `u128`.
pub mod json_safe_u128 {
    use serde::de::{self, Deserializer, Visitor};
    use serde::ser::Serializer;

    use super::JS_MAX_SAFE_INTEGER;

    pub fn serialize<S: Serializer>(value: &u128, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() && *value > JS_MAX_SAFE_INTEGER as u128 {
            serializer.serialize_str(&value.to_string())
        } else {
            serializer.serialize_u128(*value)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u128, D::Error> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_any(U128OrStringVisitor)
        } else {
            serde::Deserialize::deserialize(deserializer)
        }
    }

    struct U128OrStringVisitor;

    impl<'de> Visitor<'de> for U128OrStringVisitor {
        type Value = u128;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a u128 or a string containing a u128")
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
            Ok(v as u128)
        }

        fn visit_u128<E: de::Error>(self, v: u128) -> Result<Self::Value, E> {
            Ok(v)
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
            u128::try_from(v)
                .map_err(|_| de::Error::custom(format!("i64 value {v} out of u128 range")))
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            v.parse::<u128>()
                .map_err(|_| de::Error::custom(format!("invalid u128 string: {v}")))
        }
    }
}

/// Serde `with` module for a `u128` field that is buffered through serde's
/// `Content` enum — i.e. a field of an internally-tagged (`#[serde(tag = "…")]`)
/// enum or struct.
///
/// Same JS-safety as [`json_safe_u128`] (values above `Number.MAX_SAFE_INTEGER`
/// stringify in human-readable JSON), but it **never** emits `serialize_u128`.
/// serde's `Content` enum cannot hold a 128-bit integer in this serde version, so
/// `json_safe_u128`'s `serialize_u128` round-trips to an "invalid type: integer …
/// as u128" error once internal tagging buffers it. This variant instead encodes a
/// plain number while the value fits in `u64`, and a string once it doesn't (and,
/// in human-readable JSON, once it exceeds `Number.MAX_SAFE_INTEGER`). The
/// `Value` / bincode paths keep the value lossless via the same number/string split.
pub mod json_safe_u128_content {
    use serde::de::{self, Visitor};
    use serde::{Deserializer, Serializer};

    use super::JS_MAX_SAFE_INTEGER;

    pub fn serialize<S: Serializer>(value: &u128, serializer: S) -> Result<S::Ok, S::Error> {
        let stringify_above = if serializer.is_human_readable() {
            JS_MAX_SAFE_INTEGER as u128
        } else {
            u64::MAX as u128
        };
        if *value > stringify_above {
            serializer.serialize_str(&value.to_string())
        } else {
            serializer.serialize_u64(*value as u64)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u128, D::Error> {
        struct V;
        impl Visitor<'_> for V {
            type Value = u128;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a u128 as a number or string")
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<u128, E> {
                Ok(v as u128)
            }
            fn visit_u128<E: de::Error>(self, v: u128) -> Result<u128, E> {
                Ok(v)
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<u128, E> {
                v.parse()
                    .map_err(|_| de::Error::custom(format!("invalid u128 string: {v}")))
            }
        }
        deserializer.deserialize_any(V)
    }
}

/// Serde `with` module for `i64` fields.
pub mod json_safe_i64 {
    use serde::de::{self, Deserializer, Visitor};
    use serde::ser::Serializer;

    use super::JS_MAX_SAFE_INTEGER;

    const JS_MIN_SAFE_INTEGER: i64 = -(JS_MAX_SAFE_INTEGER as i64);

    pub fn serialize<S: Serializer>(value: &i64, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable()
            && (*value > JS_MAX_SAFE_INTEGER as i64 || *value < JS_MIN_SAFE_INTEGER)
        {
            serializer.serialize_str(&value.to_string())
        } else {
            serializer.serialize_i64(*value)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<i64, D::Error> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_any(I64OrStringVisitor)
        } else {
            serde::Deserialize::deserialize(deserializer)
        }
    }

    struct I64OrStringVisitor;

    impl<'de> Visitor<'de> for I64OrStringVisitor {
        type Value = i64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an i64 or a string containing an i64")
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
            Ok(v)
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
            i64::try_from(v)
                .map_err(|_| de::Error::custom(format!("u64 value {v} out of i64 range")))
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            v.parse::<i64>()
                .map_err(|_| de::Error::custom(format!("invalid i64 string: {v}")))
        }
    }
}

/// Serde `with` module for `Option<u64>` fields.
pub mod json_safe_option_u64 {
    use serde::de::{self, Deserializer, Visitor};
    use serde::ser::Serializer;

    pub fn serialize<S: Serializer>(value: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error> {
        match value {
            Some(v) => super::json_safe_u64::serialize(v, serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<u64>, D::Error> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_option(OptionU64Visitor)
        } else {
            serde::Deserialize::deserialize(deserializer)
        }
    }

    struct OptionU64Visitor;

    impl<'de> Visitor<'de> for OptionU64Visitor {
        type Value = Option<u64>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("null, a u64, or a string containing a u64")
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_some<D: Deserializer<'de>>(
            self,
            deserializer: D,
        ) -> Result<Self::Value, D::Error> {
            super::json_safe_u64::deserialize(deserializer).map(Some)
        }
    }
}

/// Serde `with` module for `Option<i64>` fields.
pub mod json_safe_option_i64 {
    use serde::de::{self, Deserializer, Visitor};
    use serde::ser::Serializer;

    pub fn serialize<S: Serializer>(value: &Option<i64>, serializer: S) -> Result<S::Ok, S::Error> {
        match value {
            Some(v) => super::json_safe_i64::serialize(v, serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<i64>, D::Error> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_option(OptionI64Visitor)
        } else {
            serde::Deserialize::deserialize(deserializer)
        }
    }

    struct OptionI64Visitor;

    impl<'de> Visitor<'de> for OptionI64Visitor {
        type Value = Option<i64>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("null, an i64, or a string containing an i64")
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_some<D: Deserializer<'de>>(
            self,
            deserializer: D,
        ) -> Result<Self::Value, D::Error> {
            super::json_safe_i64::deserialize(deserializer).map(Some)
        }
    }
}

/// Serde `with` module for `Option<(String, u64)>` fields.
///
/// Used by `DocumentCreateTransitionV0::prefunded_voting_balance`. The
/// `json_safe_fields` macro can't auto-inject on tuple-inside-Option fields,
/// so this is added explicitly via `serde(with = ...)`. JS-safety semantics
/// match `json_safe_u64`: large u64 values become strings in HR; non-HR
/// keeps native u64.
pub mod json_safe_option_string_u64_tuple {
    use serde::de::{self, Deserializer, SeqAccess, Visitor};
    use serde::ser::{SerializeTuple, Serializer};

    pub fn serialize<S: Serializer>(
        value: &Option<(String, u64)>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some((s, n)) => {
                let stringify = serializer.is_human_readable() && *n > super::JS_MAX_SAFE_INTEGER;
                let mut tup = serializer.serialize_tuple(2)?;
                tup.serialize_element(s)?;
                if stringify {
                    tup.serialize_element(&n.to_string())?;
                } else {
                    tup.serialize_element(n)?;
                }
                tup.end()
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<(String, u64)>, D::Error> {
        deserializer.deserialize_option(OptStringU64TupleVisitor)
    }

    struct OptStringU64TupleVisitor;

    impl<'de> Visitor<'de> for OptStringU64TupleVisitor {
        type Value = Option<(String, u64)>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("null or a 2-tuple [String, u64-or-string]")
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_some<D: Deserializer<'de>>(
            self,
            deserializer: D,
        ) -> Result<Self::Value, D::Error> {
            deserializer
                .deserialize_tuple(2, StringU64TupleVisitor)
                .map(Some)
        }

        // Some self-describing formats (serde_json with deserialize_any) call
        // visit_seq directly when the wire shape is an array — accept that too.
        fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
            StringU64TupleVisitor.visit_seq(seq).map(Some)
        }
    }

    /// Newtype wrapper that delegates u64 deserialization to `json_safe_u64`,
    /// accepting both numbers and strings in HR.
    #[derive(serde::Deserialize)]
    #[serde(transparent)]
    struct SafeU64(#[serde(with = "super::json_safe_u64")] u64);

    struct StringU64TupleVisitor;

    impl<'de> Visitor<'de> for StringU64TupleVisitor {
        type Value = (String, u64);

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a 2-tuple [String, u64-or-string]")
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let s: String = seq
                .next_element()?
                .ok_or_else(|| de::Error::invalid_length(0, &"a 2-tuple"))?;
            let n: SafeU64 = seq
                .next_element()?
                .ok_or_else(|| de::Error::invalid_length(1, &"a 2-tuple"))?;
            Ok((s, n.0))
        }
    }
}

/// Serde `with` module for `Option<(u32, u32, Vec<u8>)>` fields used by the
/// `SharedEncryptedNote` / `PrivateEncryptedNote` type aliases on token
/// transitions.
///
/// In HR (JSON) the inner `Vec<u8>` is base64-encoded so the wire shape is
/// `[u32, u32, "<base64>"]` instead of an array-of-numbers. In non-HR
/// (platform_value, bincode) the bytes stay as raw bytes (`Value::Bytes`).
/// The two `u32` indices are always JS-safe (well below `MAX_SAFE_INTEGER`)
/// so they don't need special protection.
pub mod json_safe_option_encrypted_note {
    use serde::de::{self, Deserializer, SeqAccess, Visitor};
    use serde::ser::{SerializeTuple, Serializer};

    /// Wrapper that emits its byte payload via `serialize_bytes` (raw bytes)
    /// rather than the default `Vec<u8>` Serialize (sequence of u8). Used in
    /// the non-HR path so platform_value receives `Value::Bytes` and bincode
    /// emits a length-prefixed byte buffer.
    struct BytesAsBytes<'a>(&'a [u8]);

    impl<'a> serde::Serialize for BytesAsBytes<'a> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            s.serialize_bytes(self.0)
        }
    }

    pub fn serialize<S: Serializer>(
        value: &Option<(u32, u32, Vec<u8>)>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some((a, b, bytes)) => {
                let is_hr = serializer.is_human_readable();
                let mut tup = serializer.serialize_tuple(3)?;
                tup.serialize_element(a)?;
                tup.serialize_element(b)?;
                if is_hr {
                    use base64::Engine;
                    let s = base64::engine::general_purpose::STANDARD.encode(bytes);
                    tup.serialize_element(&s)?;
                } else {
                    tup.serialize_element(&BytesAsBytes(bytes))?;
                }
                tup.end()
            }
            None => serializer.serialize_none(),
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<(u32, u32, Vec<u8>)>, D::Error> {
        deserializer.deserialize_option(OptEncryptedNoteVisitor)
    }

    struct OptEncryptedNoteVisitor;

    impl<'de> Visitor<'de> for OptEncryptedNoteVisitor {
        type Value = Option<(u32, u32, Vec<u8>)>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("null or a 3-tuple [u32, u32, base64-string-or-bytes]")
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_some<D: Deserializer<'de>>(
            self,
            deserializer: D,
        ) -> Result<Self::Value, D::Error> {
            deserializer
                .deserialize_tuple(3, EncryptedNoteVisitor)
                .map(Some)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
            EncryptedNoteVisitor.visit_seq(seq).map(Some)
        }
    }

    /// Newtype wrapper that accepts either a base64 string (HR) or a byte
    /// sequence (non-HR) and produces a `Vec<u8>`.
    struct BytesField(Vec<u8>);

    impl<'de> serde::Deserialize<'de> for BytesField {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            struct V;
            impl<'de> Visitor<'de> for V {
                type Value = Vec<u8>;

                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str("base64 string or byte sequence")
                }

                fn visit_str<E: de::Error>(self, s: &str) -> Result<Vec<u8>, E> {
                    use base64::Engine;
                    base64::engine::general_purpose::STANDARD
                        .decode(s)
                        .map_err(|e| E::custom(format!("invalid base64: {e}")))
                }

                fn visit_bytes<E: de::Error>(self, b: &[u8]) -> Result<Vec<u8>, E> {
                    Ok(b.to_vec())
                }

                fn visit_byte_buf<E: de::Error>(self, b: Vec<u8>) -> Result<Vec<u8>, E> {
                    Ok(b)
                }

                fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<u8>, A::Error> {
                    let mut out = Vec::new();
                    while let Some(b) = seq.next_element::<u8>()? {
                        out.push(b);
                    }
                    Ok(out)
                }
            }
            // Use `deserialize_any` so we accept whichever path the deserializer
            // takes (string for JSON, bytes for bincode/platform_value).
            d.deserialize_any(V).map(BytesField)
        }
    }

    struct EncryptedNoteVisitor;

    impl<'de> Visitor<'de> for EncryptedNoteVisitor {
        type Value = (u32, u32, Vec<u8>);

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a 3-tuple [u32, u32, base64-string-or-bytes]")
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let a: u32 = seq
                .next_element()?
                .ok_or_else(|| de::Error::invalid_length(0, &"a 3-tuple"))?;
            let b: u32 = seq
                .next_element()?
                .ok_or_else(|| de::Error::invalid_length(1, &"a 3-tuple"))?;
            let bytes: BytesField = seq
                .next_element()?
                .ok_or_else(|| de::Error::invalid_length(2, &"a 3-tuple"))?;
            Ok((a, b, bytes.0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestU64 {
        #[serde(with = "json_safe_u64")]
        value: u64,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestU128 {
        #[serde(with = "json_safe_u128")]
        value: u128,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestI64 {
        #[serde(with = "json_safe_i64")]
        value: i64,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestOptionU64 {
        #[serde(default, with = "json_safe_option_u64")]
        value: Option<u64>,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestOptionI64 {
        #[serde(default, with = "json_safe_option_i64")]
        value: Option<i64>,
    }

    #[test]
    fn u64_small_value_stays_number() {
        let t = TestU64 { value: 42 };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_number());
        assert_eq!(json["value"].as_u64().unwrap(), 42);

        let restored: TestU64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn u64_large_value_becomes_string() {
        let t = TestU64 { value: u64::MAX };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_string());
        assert_eq!(json["value"].as_str().unwrap(), "18446744073709551615");

        let restored: TestU64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn u128_small_value_stays_number() {
        let t = TestU128 { value: 42 };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_number());
        assert_eq!(json["value"].as_u64().unwrap(), 42);

        let restored: TestU128 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn u128_large_value_becomes_string() {
        // Above u64::MAX — only representable as a string in JS-safe JSON.
        let t = TestU128 {
            value: (u64::MAX as u128) + 1,
        };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_string());
        assert_eq!(json["value"].as_str().unwrap(), "18446744073709551616");

        let restored: TestU128 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn u128_value_round_trips_through_non_human_readable() {
        // platform_value is non-human-readable → native u128, no stringification.
        let t = TestU128 {
            value: (u64::MAX as u128) + 12345,
        };
        let value = platform_value::to_value(&t).unwrap();
        let restored: TestU128 = platform_value::from_value(value).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn u64_at_max_safe_integer_stays_number() {
        let t = TestU64 {
            value: JS_MAX_SAFE_INTEGER,
        };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_number());
    }

    #[test]
    fn u64_above_max_safe_integer_becomes_string() {
        let t = TestU64 {
            value: JS_MAX_SAFE_INTEGER + 1,
        };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_string());
    }

    #[test]
    fn i64_small_value_stays_number() {
        let t = TestI64 { value: -42 };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_number());

        let restored: TestI64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn i64_large_value_becomes_string() {
        let t = TestI64 { value: i64::MAX };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_string());

        let restored: TestI64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn i64_large_negative_becomes_string() {
        let t = TestI64 { value: i64::MIN };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_string());

        let restored: TestI64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn option_u64_none_round_trip() {
        let t = TestOptionU64 { value: None };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_null());

        let restored: TestOptionU64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn option_u64_large_round_trip() {
        let t = TestOptionU64 {
            value: Some(u64::MAX),
        };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_string());

        let restored: TestOptionU64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn platform_value_u64_stays_native() {
        let t = TestU64 { value: u64::MAX };
        let pv = platform_value::to_value(&t).unwrap();

        // platform_value is non-human-readable, so u64 stays as u64
        let restored: TestU64 = platform_value::from_value(pv).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn option_i64_none_round_trip() {
        let t = TestOptionI64 { value: None };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_null());

        let restored: TestOptionI64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn option_i64_large_round_trip() {
        let t = TestOptionI64 {
            value: Some(i64::MAX),
        };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_string());

        let restored: TestOptionI64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn option_i64_large_negative_round_trip() {
        let t = TestOptionI64 {
            value: Some(i64::MIN),
        };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_string());

        let restored: TestOptionI64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn option_i64_missing_field_deserializes_as_none() {
        let json = serde_json::json!({});
        let restored: TestOptionI64 = serde_json::from_value(json).unwrap();
        assert_eq!(restored.value, None);
    }

    #[test]
    fn option_u64_missing_field_deserializes_as_none() {
        let json = serde_json::json!({});
        let restored: TestOptionU64 = serde_json::from_value(json).unwrap();
        assert_eq!(restored.value, None);
    }

    #[test]
    fn u64_deserialize_from_i64_value() {
        // Tests visit_i64 path: JSON number that fits in i64 parsed as u64
        let json = serde_json::json!({"value": 42});
        let restored: TestU64 = serde_json::from_value(json).unwrap();
        assert_eq!(restored.value, 42);
    }

    #[test]
    fn u64_deserialize_negative_i64_fails() {
        // Tests visit_i64 error path: negative i64 can't become u64
        let json = serde_json::json!({"value": -1});
        let result = serde_json::from_value::<TestU64>(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("out of u64 range"));
    }

    #[test]
    fn u64_deserialize_invalid_string_fails() {
        let json = serde_json::json!({"value": "not_a_number"});
        let result = serde_json::from_value::<TestU64>(json);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid u64 string"));
    }

    #[test]
    fn i64_deserialize_u64_overflow_fails() {
        // Tests visit_u64 error path: u64::MAX can't fit in i64
        let json = serde_json::json!({"value": u64::MAX.to_string()});
        // This goes through visit_str which parses as i64 — will fail
        let result = serde_json::from_value::<TestI64>(json);
        assert!(result.is_err());
    }

    #[test]
    fn i64_deserialize_invalid_string_fails() {
        let json = serde_json::json!({"value": "not_a_number"});
        let result = serde_json::from_value::<TestI64>(json);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid i64 string"));
    }

    #[test]
    fn platform_value_i64_stays_native() {
        let t = TestI64 { value: i64::MAX };
        let pv = platform_value::to_value(&t).unwrap();
        let restored: TestI64 = platform_value::from_value(pv).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn platform_value_option_u64_round_trip() {
        let t = TestOptionU64 {
            value: Some(u64::MAX),
        };
        let pv = platform_value::to_value(&t).unwrap();
        let restored: TestOptionU64 = platform_value::from_value(pv).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn platform_value_option_i64_round_trip() {
        let t = TestOptionI64 {
            value: Some(i64::MIN),
        };
        let pv = platform_value::to_value(&t).unwrap();
        let restored: TestOptionI64 = platform_value::from_value(pv).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn option_u64_small_value_stays_number() {
        let t = TestOptionU64 { value: Some(42) };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_number());

        let restored: TestOptionU64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn option_i64_small_value_stays_number() {
        let t = TestOptionI64 { value: Some(-42) };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_number());

        let restored: TestOptionI64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn tagged_enum_with_u64_round_trip() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        #[serde(tag = "$formatVersion")]
        enum Versioned {
            #[serde(rename = "0")]
            V0(TestU64),
        }

        let v = Versioned::V0(TestU64 { value: u64::MAX });
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(json["$formatVersion"], "0");
        assert!(json["value"].is_string());

        let restored: Versioned = serde_json::from_value(json).unwrap();
        assert_eq!(v, restored);
    }

    // --- Additional edge-case tests for json_safe_option_i64 ---

    #[test]
    fn option_i64_some_safe_value_stays_number() {
        let t = TestOptionI64 { value: Some(1000) };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_number());
        assert_eq!(json["value"].as_i64().unwrap(), 1000);

        let restored: TestOptionI64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn option_i64_some_unsafe_positive_becomes_string() {
        // JS_MAX_SAFE_INTEGER + 1 as i64
        let unsafe_val = (JS_MAX_SAFE_INTEGER + 1) as i64;
        let t = TestOptionI64 {
            value: Some(unsafe_val),
        };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_string());

        let restored: TestOptionI64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn option_i64_some_unsafe_negative_becomes_string() {
        // -(JS_MAX_SAFE_INTEGER) - 1 is below the safe boundary
        let unsafe_neg = -(JS_MAX_SAFE_INTEGER as i64) - 1;
        let t = TestOptionI64 {
            value: Some(unsafe_neg),
        };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_string());

        let restored: TestOptionI64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    // --- Boundary tests ---

    #[test]
    fn u64_exactly_at_max_safe_integer_round_trip() {
        let t = TestU64 {
            value: JS_MAX_SAFE_INTEGER,
        };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_number());

        let restored: TestU64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn u64_one_above_max_safe_integer_round_trip() {
        let t = TestU64 {
            value: JS_MAX_SAFE_INTEGER + 1,
        };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_string());
        assert_eq!(
            json["value"].as_str().unwrap(),
            (JS_MAX_SAFE_INTEGER + 1).to_string()
        );

        let restored: TestU64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn i64_exactly_at_positive_safe_boundary_stays_number() {
        let t = TestI64 {
            value: JS_MAX_SAFE_INTEGER as i64,
        };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_number());

        let restored: TestI64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn i64_one_above_positive_safe_boundary_becomes_string() {
        let t = TestI64 {
            value: JS_MAX_SAFE_INTEGER as i64 + 1,
        };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_string());

        let restored: TestI64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn i64_exactly_at_negative_safe_boundary_stays_number() {
        let t = TestI64 {
            value: -(JS_MAX_SAFE_INTEGER as i64),
        };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_number());

        let restored: TestI64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn i64_one_below_negative_safe_boundary_becomes_string() {
        let t = TestI64 {
            value: -(JS_MAX_SAFE_INTEGER as i64) - 1,
        };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_string());

        let restored: TestI64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    // --- Zero and negative value tests ---

    #[test]
    fn u64_zero_stays_number() {
        let t = TestU64 { value: 0 };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_number());
        assert_eq!(json["value"].as_u64().unwrap(), 0);

        let restored: TestU64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn i64_zero_stays_number() {
        let t = TestI64 { value: 0 };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_number());
        assert_eq!(json["value"].as_i64().unwrap(), 0);

        let restored: TestI64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn i64_negative_one_stays_number() {
        let t = TestI64 { value: -1 };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_number());

        let restored: TestI64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn option_i64_zero_round_trip() {
        let t = TestOptionI64 { value: Some(0) };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_number());

        let restored: TestOptionI64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn option_u64_zero_round_trip() {
        let t = TestOptionU64 { value: Some(0) };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_number());

        let restored: TestOptionU64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn option_u64_at_max_safe_integer_stays_number() {
        let t = TestOptionU64 {
            value: Some(JS_MAX_SAFE_INTEGER),
        };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_number());

        let restored: TestOptionU64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn option_u64_above_max_safe_integer_becomes_string() {
        let t = TestOptionU64 {
            value: Some(JS_MAX_SAFE_INTEGER + 1),
        };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_string());

        let restored: TestOptionU64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn option_i64_at_positive_safe_boundary_stays_number() {
        let t = TestOptionI64 {
            value: Some(JS_MAX_SAFE_INTEGER as i64),
        };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_number());

        let restored: TestOptionI64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn option_i64_above_positive_safe_boundary_becomes_string() {
        let t = TestOptionI64 {
            value: Some(JS_MAX_SAFE_INTEGER as i64 + 1),
        };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_string());

        let restored: TestOptionI64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn option_i64_at_negative_safe_boundary_stays_number() {
        let t = TestOptionI64 {
            value: Some(-(JS_MAX_SAFE_INTEGER as i64)),
        };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_number());

        let restored: TestOptionI64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn option_i64_below_negative_safe_boundary_becomes_string() {
        let t = TestOptionI64 {
            value: Some(-(JS_MAX_SAFE_INTEGER as i64) - 1),
        };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["value"].is_string());

        let restored: TestOptionI64 = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn platform_value_option_i64_none_round_trip() {
        let t = TestOptionI64 { value: None };
        let pv = platform_value::to_value(&t).unwrap();
        let restored: TestOptionI64 = platform_value::from_value(pv).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn platform_value_option_u64_none_round_trip() {
        let t = TestOptionU64 { value: None };
        let pv = platform_value::to_value(&t).unwrap();
        let restored: TestOptionU64 = platform_value::from_value(pv).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn u64_deserialize_from_string_number() {
        // Deserialize a string-encoded number (even one that fits in a number)
        let json = serde_json::json!({"value": "42"});
        let restored: TestU64 = serde_json::from_value(json).unwrap();
        assert_eq!(restored.value, 42);
    }

    #[test]
    fn i64_deserialize_from_string_number() {
        let json = serde_json::json!({"value": "-12345"});
        let restored: TestI64 = serde_json::from_value(json).unwrap();
        assert_eq!(restored.value, -12345);
    }
}
