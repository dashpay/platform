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
}
