//! Serde `with` modules for `BTreeMap` fields containing u64 keys and/or values.
//!
//! Unlike bare u64 fields (handled automatically by `#[json_safe_fields]`), map
//! fields need explicit `#[serde(with = "...")]` annotations because the macro
//! cannot inject `serde(with)` into generic container internals.
//!
//! ## When to use
//!
//! If a `#[json_safe_fields]`-annotated struct has a `BTreeMap<K, u64>` field,
//! the compile-time `JsonSafeFields` check will fail (u64 doesn't implement the
//! trait). Add one of these modules as a `serde(with)` annotation to fix it.
//!
//! ## Available modules
//!
//! - [`json_safe_u64_u64_map`] — `BTreeMap<u64, u64>`
//! - [`json_safe_identifier_u64_map`] — `BTreeMap<Identifier, u64>`
//! - [`json_safe_generic_u64_value_map`] — `BTreeMap<K, u64>` for any serializable key
//! - [`json_safe_u64_nested_identifier_u64_map`] — `BTreeMap<u64, BTreeMap<Identifier, u64>>`
//!
//! ## Behavior
//!
//! Same as `safe_integer.rs`: uses `is_human_readable()` to only stringify in
//! JSON mode. platform_value and bincode stay native.

use super::safe_integer::JS_MAX_SAFE_INTEGER;

/// Serde `with` module for `BTreeMap<u64, u64>` fields.
///
/// - Keys: JSON map keys are always strings, so keys are inherently safe.
///   Deserialization accepts both string and numeric keys.
/// - Values: Large u64 values are serialized as strings in JSON.
/// - Non-HR (platform_value): native u64 for both keys and values.
pub mod json_safe_u64_u64_map {
    use serde::de::{self, Deserializer, MapAccess, Visitor};
    use serde::ser::{SerializeMap, Serializer};
    use std::collections::BTreeMap;

    use super::JS_MAX_SAFE_INTEGER;

    pub fn serialize<S: Serializer>(
        map: &BTreeMap<u64, u64>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            let mut s = serializer.serialize_map(Some(map.len()))?;
            for (k, v) in map {
                // JSON keys are always strings
                s.serialize_entry(
                    &k.to_string(),
                    &if *v > JS_MAX_SAFE_INTEGER {
                        serde_json::Value::String(v.to_string())
                    } else {
                        serde_json::Value::Number((*v).into())
                    },
                )?;
            }
            s.end()
        } else {
            serde::Serialize::serialize(map, serializer)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<BTreeMap<u64, u64>, D::Error> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_map(U64U64MapVisitor)
        } else {
            serde::Deserialize::deserialize(deserializer)
        }
    }

    struct U64U64MapVisitor;

    impl<'de> Visitor<'de> for U64U64MapVisitor {
        type Value = BTreeMap<u64, u64>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a map with u64 keys and u64 values (numbers or strings)")
        }

        fn visit_map<M: MapAccess<'de>>(self, mut access: M) -> Result<Self::Value, M::Error> {
            let mut map = BTreeMap::new();
            while let Some((key, value)) =
                access.next_entry::<serde_json::Value, serde_json::Value>()?
            {
                let k = json_value_to_u64(&key).map_err(de::Error::custom)?;
                let v = json_value_to_u64(&value).map_err(de::Error::custom)?;
                map.insert(k, v);
            }
            Ok(map)
        }
    }

    fn json_value_to_u64(v: &serde_json::Value) -> Result<u64, String> {
        match v {
            serde_json::Value::Number(n) => n
                .as_u64()
                .ok_or_else(|| format!("expected u64 number, got: {n}")),
            serde_json::Value::String(s) => s
                .parse::<u64>()
                .map_err(|_| format!("invalid u64 string: {s}")),
            other => Err(format!("expected u64 or string, got: {other}")),
        }
    }
}

/// Serde `with` module for `BTreeMap<Identifier, u64>` fields.
///
/// - Keys: Identifier (not u64, uses its own serde impl).
/// - Values: Large u64 values are serialized as strings in JSON.
/// - Non-HR (platform_value): native serialization.
pub mod json_safe_identifier_u64_map {
    use platform_value::Identifier;
    use serde::de::{self, Deserializer, MapAccess, Visitor};
    use serde::ser::{SerializeMap, Serializer};
    use std::collections::BTreeMap;

    use super::JS_MAX_SAFE_INTEGER;

    pub fn serialize<S: Serializer>(
        map: &BTreeMap<Identifier, u64>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            let mut s = serializer.serialize_map(Some(map.len()))?;
            for (k, v) in map {
                s.serialize_entry(
                    k,
                    &if *v > JS_MAX_SAFE_INTEGER {
                        serde_json::Value::String(v.to_string())
                    } else {
                        serde_json::Value::Number((*v).into())
                    },
                )?;
            }
            s.end()
        } else {
            serde::Serialize::serialize(map, serializer)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<BTreeMap<Identifier, u64>, D::Error> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_map(IdentifierU64MapVisitor)
        } else {
            serde::Deserialize::deserialize(deserializer)
        }
    }

    struct IdentifierU64MapVisitor;

    impl<'de> Visitor<'de> for IdentifierU64MapVisitor {
        type Value = BTreeMap<Identifier, u64>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a map with Identifier keys and u64 values (numbers or strings)")
        }

        fn visit_map<M: MapAccess<'de>>(self, mut access: M) -> Result<Self::Value, M::Error> {
            let mut map = BTreeMap::new();
            while let Some((key, value)) = access.next_entry::<Identifier, serde_json::Value>()? {
                let v = match &value {
                    serde_json::Value::Number(n) => n
                        .as_u64()
                        .ok_or_else(|| de::Error::custom(format!("expected u64 number, got: {n}"))),
                    serde_json::Value::String(s) => s
                        .parse::<u64>()
                        .map_err(|_| de::Error::custom(format!("invalid u64 string: {s}"))),
                    other => Err(de::Error::custom(format!(
                        "expected u64 or string, got: {other}"
                    ))),
                }?;
                map.insert(key, v);
            }
            Ok(map)
        }
    }
}

/// Serde `with` module for `BTreeMap<u64, BTreeMap<Identifier, u64>>` fields.
///
/// - Outer keys: u64 (JSON keys always strings, accept both).
/// - Inner keys: Identifier.
/// - Inner values: u64, stringified when large.
/// - Non-HR (platform_value): native serialization.
pub mod json_safe_u64_nested_identifier_u64_map {
    use platform_value::Identifier;
    use serde::de::{self, Deserializer, MapAccess, Visitor};
    use serde::ser::{SerializeMap, Serializer};
    use std::collections::BTreeMap;

    use super::JS_MAX_SAFE_INTEGER;

    pub fn serialize<S: Serializer>(
        map: &BTreeMap<u64, BTreeMap<Identifier, u64>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            let mut outer = serializer.serialize_map(Some(map.len()))?;
            for (k, inner) in map {
                let safe_inner: BTreeMap<&Identifier, serde_json::Value> = inner
                    .iter()
                    .map(|(ik, iv)| {
                        let v = if *iv > JS_MAX_SAFE_INTEGER {
                            serde_json::Value::String(iv.to_string())
                        } else {
                            serde_json::Value::Number((*iv).into())
                        };
                        (ik, v)
                    })
                    .collect();
                outer.serialize_entry(&k.to_string(), &safe_inner)?;
            }
            outer.end()
        } else {
            serde::Serialize::serialize(map, serializer)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<BTreeMap<u64, BTreeMap<Identifier, u64>>, D::Error> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_map(OuterMapVisitor)
        } else {
            serde::Deserialize::deserialize(deserializer)
        }
    }

    struct OuterMapVisitor;

    impl<'de> Visitor<'de> for OuterMapVisitor {
        type Value = BTreeMap<u64, BTreeMap<Identifier, u64>>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a nested map: u64 -> (Identifier -> u64)")
        }

        fn visit_map<M: MapAccess<'de>>(self, mut access: M) -> Result<Self::Value, M::Error> {
            let mut map = BTreeMap::new();
            while let Some((key_str, inner_json)) =
                access.next_entry::<serde_json::Value, BTreeMap<Identifier, serde_json::Value>>()?
            {
                let k = match &key_str {
                    serde_json::Value::Number(n) => n
                        .as_u64()
                        .ok_or_else(|| de::Error::custom(format!("expected u64 key, got: {n}"))),
                    serde_json::Value::String(s) => s
                        .parse::<u64>()
                        .map_err(|_| de::Error::custom(format!("invalid u64 key: {s}"))),
                    other => Err(de::Error::custom(format!("expected u64 key, got: {other}"))),
                }?;

                let inner: BTreeMap<Identifier, u64> = inner_json
                    .into_iter()
                    .map(|(ik, iv)| {
                        let v = match &iv {
                            serde_json::Value::Number(n) => n.as_u64().ok_or_else(|| {
                                de::Error::custom(format!("expected u64 value, got: {n}"))
                            }),
                            serde_json::Value::String(s) => s
                                .parse::<u64>()
                                .map_err(|_| de::Error::custom(format!("invalid u64 string: {s}"))),
                            other => Err(de::Error::custom(format!(
                                "expected u64 or string, got: {other}"
                            ))),
                        }?;
                        Ok((ik, v))
                    })
                    .collect::<Result<_, M::Error>>()?;

                map.insert(k, inner);
            }
            Ok(map)
        }
    }
}

/// Generic serde `with` module for `BTreeMap<K, u64>` fields where K is any
/// serializable/deserializable key type.
///
/// - Keys: Use their own serde impl (unchanged).
/// - Values: Large u64 values are serialized as strings in JSON.
/// - Non-HR (platform_value): native serialization.
pub mod json_safe_generic_u64_value_map {
    use serde::de::{self, Deserializer, MapAccess, Visitor};
    use serde::ser::{SerializeMap, Serializer};
    use std::collections::BTreeMap;
    use std::marker::PhantomData;

    use super::JS_MAX_SAFE_INTEGER;

    pub fn serialize<K, S>(map: &BTreeMap<K, u64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        K: serde::Serialize + Ord,
        S: Serializer,
    {
        if serializer.is_human_readable() {
            let mut s = serializer.serialize_map(Some(map.len()))?;
            for (k, v) in map {
                s.serialize_entry(
                    k,
                    &if *v > JS_MAX_SAFE_INTEGER {
                        serde_json::Value::String(v.to_string())
                    } else {
                        serde_json::Value::Number((*v).into())
                    },
                )?;
            }
            s.end()
        } else {
            serde::Serialize::serialize(map, serializer)
        }
    }

    pub fn deserialize<'de, K, D>(deserializer: D) -> Result<BTreeMap<K, u64>, D::Error>
    where
        K: serde::Deserialize<'de> + Ord,
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            deserializer.deserialize_map(GenericU64ValueMapVisitor(PhantomData))
        } else {
            serde::Deserialize::deserialize(deserializer)
        }
    }

    struct GenericU64ValueMapVisitor<K>(PhantomData<K>);

    impl<'de, K> Visitor<'de> for GenericU64ValueMapVisitor<K>
    where
        K: serde::Deserialize<'de> + Ord,
    {
        type Value = BTreeMap<K, u64>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a map with u64 values (numbers or strings)")
        }

        fn visit_map<M: MapAccess<'de>>(self, mut access: M) -> Result<Self::Value, M::Error> {
            let mut map = BTreeMap::new();
            while let Some((key, value)) = access.next_entry::<K, serde_json::Value>()? {
                let v = match &value {
                    serde_json::Value::Number(n) => n
                        .as_u64()
                        .ok_or_else(|| de::Error::custom(format!("expected u64 number, got: {n}"))),
                    serde_json::Value::String(s) => s
                        .parse::<u64>()
                        .map_err(|_| de::Error::custom(format!("invalid u64 string: {s}"))),
                    other => Err(de::Error::custom(format!(
                        "expected u64 or string, got: {other}"
                    ))),
                }?;
                map.insert(key, v);
            }
            Ok(map)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_value::string_encoding::Encoding;
    use platform_value::Identifier;
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestIdentifierU64Map {
        #[serde(with = "json_safe_identifier_u64_map")]
        data: BTreeMap<Identifier, u64>,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestU64U64Map {
        #[serde(with = "json_safe_u64_u64_map")]
        data: BTreeMap<u64, u64>,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestNestedMap {
        #[serde(with = "json_safe_u64_nested_identifier_u64_map")]
        data: BTreeMap<u64, BTreeMap<Identifier, u64>>,
    }

    #[test]
    fn identifier_u64_map_small_values_stay_numbers() {
        let id = Identifier::random();
        let mut data = BTreeMap::new();
        data.insert(id, 42u64);
        let t = TestIdentifierU64Map { data };
        let json = serde_json::to_value(&t).unwrap();
        let restored: TestIdentifierU64Map = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn identifier_u64_map_large_values_become_strings() {
        let id = Identifier::random();
        let mut data = BTreeMap::new();
        data.insert(id, u64::MAX);
        let t = TestIdentifierU64Map { data };
        let json = serde_json::to_value(&t).unwrap();

        // The value should be a string
        let map_obj = json["data"].as_object().unwrap();
        let val = map_obj.values().next().unwrap();
        assert!(val.is_string());
        assert_eq!(val.as_str().unwrap(), "18446744073709551615");

        let restored: TestIdentifierU64Map = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn u64_u64_map_round_trip_with_large_values() {
        let mut data = BTreeMap::new();
        data.insert(100u64, 42u64);
        data.insert(u64::MAX, u64::MAX);
        let t = TestU64U64Map { data };
        let json = serde_json::to_value(&t).unwrap();

        // Large value should be stringified
        let map_obj = json["data"].as_object().unwrap();
        let large_val = &map_obj["18446744073709551615"];
        assert!(large_val.is_string());

        let restored: TestU64U64Map = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
    struct CustomKey(String);

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestGenericMap {
        #[serde(with = "json_safe_generic_u64_value_map")]
        data: BTreeMap<CustomKey, u64>,
    }

    #[test]
    fn generic_map_small_values_stay_numbers() {
        let mut data = BTreeMap::new();
        data.insert(CustomKey("alice".into()), 42u64);
        let t = TestGenericMap { data };
        let json = serde_json::to_value(&t).unwrap();

        let val = &json["data"]["alice"];
        assert!(val.is_number());
        assert_eq!(val.as_u64().unwrap(), 42);

        let restored: TestGenericMap = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn generic_map_large_values_become_strings() {
        let mut data = BTreeMap::new();
        data.insert(CustomKey("bob".into()), u64::MAX);
        let t = TestGenericMap { data };
        let json = serde_json::to_value(&t).unwrap();

        let val = &json["data"]["bob"];
        assert!(val.is_string());
        assert_eq!(val.as_str().unwrap(), "18446744073709551615");

        let restored: TestGenericMap = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn generic_map_mixed_values_round_trip() {
        let mut data = BTreeMap::new();
        data.insert(CustomKey("small".into()), 100u64);
        data.insert(CustomKey("large".into()), u64::MAX);
        let t = TestGenericMap { data };
        let json = serde_json::to_value(&t).unwrap();

        // Small stays number, large becomes string
        assert!(json["data"]["small"].is_number());
        assert!(json["data"]["large"].is_string());

        let restored: TestGenericMap = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn u64_u64_map_small_values_stay_numbers() {
        let mut data = BTreeMap::new();
        data.insert(5u64, 10u64);
        let t = TestU64U64Map { data };
        let json = serde_json::to_value(&t).unwrap();
        let map_obj = json["data"].as_object().unwrap();
        let val = &map_obj["5"];
        assert!(val.is_number());

        let restored: TestU64U64Map = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn u64_u64_map_empty_round_trip() {
        let t = TestU64U64Map {
            data: BTreeMap::new(),
        };
        let json = serde_json::to_value(&t).unwrap();
        let restored: TestU64U64Map = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn identifier_u64_map_empty_round_trip() {
        let t = TestIdentifierU64Map {
            data: BTreeMap::new(),
        };
        let json = serde_json::to_value(&t).unwrap();
        let restored: TestIdentifierU64Map = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn platform_value_generic_map_round_trip() {
        let mut data = BTreeMap::new();
        data.insert(CustomKey("x".into()), u64::MAX);
        let t = TestGenericMap { data };
        let pv = platform_value::to_value(&t).unwrap();
        let restored: TestGenericMap = platform_value::from_value(pv).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn u64_u64_map_invalid_value_type_fails() {
        let json = serde_json::json!({"data": {"1": true}});
        let result = serde_json::from_value::<TestU64U64Map>(json);
        assert!(result.is_err());
    }

    #[test]
    fn u64_u64_map_invalid_value_string_fails() {
        let json = serde_json::json!({"data": {"1": "not_a_number"}});
        let result = serde_json::from_value::<TestU64U64Map>(json);
        assert!(result.is_err());
    }

    #[test]
    fn identifier_u64_map_invalid_value_type_fails() {
        let id = Identifier::random();
        let json = serde_json::json!({"data": {id.to_string(Encoding::Base58): [1, 2, 3]}});
        let result = serde_json::from_value::<TestIdentifierU64Map>(json);
        assert!(result.is_err());
    }

    #[test]
    fn nested_map_empty_inner_round_trip() {
        let mut data = BTreeMap::new();
        data.insert(1u64, BTreeMap::new());
        let t = TestNestedMap { data };
        let json = serde_json::to_value(&t).unwrap();
        let restored: TestNestedMap = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn nested_map_round_trip() {
        let id = Identifier::random();
        let mut inner = BTreeMap::new();
        inner.insert(id, u64::MAX);
        let mut data = BTreeMap::new();
        data.insert(1735689600000u64, inner);
        let t = TestNestedMap { data };
        let json = serde_json::to_value(&t).unwrap();
        let restored: TestNestedMap = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    // --- Additional tests for json_safe_u64_nested_identifier_u64_map ---

    #[test]
    fn nested_map_multiple_inner_keys_round_trip() {
        let id1 = Identifier::random();
        let id2 = Identifier::random();
        let mut inner = BTreeMap::new();
        inner.insert(id1, 42u64);
        inner.insert(id2, u64::MAX);
        let mut data = BTreeMap::new();
        data.insert(1u64, inner);
        let t = TestNestedMap { data };
        let json = serde_json::to_value(&t).unwrap();
        let restored: TestNestedMap = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn nested_map_multiple_outer_keys_round_trip() {
        let id1 = Identifier::random();
        let id2 = Identifier::random();

        let mut inner1 = BTreeMap::new();
        inner1.insert(id1, 100u64);
        let mut inner2 = BTreeMap::new();
        inner2.insert(id2, u64::MAX);

        let mut data = BTreeMap::new();
        data.insert(0u64, inner1);
        data.insert(u64::MAX, inner2);

        let t = TestNestedMap { data };
        let json = serde_json::to_value(&t).unwrap();

        // Verify outer key u64::MAX is a string key
        let map_obj = json["data"].as_object().unwrap();
        assert!(map_obj.contains_key("18446744073709551615"));

        let restored: TestNestedMap = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn nested_map_empty_outer_round_trip() {
        let t = TestNestedMap {
            data: BTreeMap::new(),
        };
        let json = serde_json::to_value(&t).unwrap();
        let restored: TestNestedMap = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn nested_map_large_outer_key_stringified_in_json() {
        let id = Identifier::random();
        let mut inner = BTreeMap::new();
        inner.insert(id, 42u64);
        let mut data = BTreeMap::new();
        data.insert(u64::MAX, inner);
        let t = TestNestedMap { data };
        let json = serde_json::to_value(&t).unwrap();
        // Outer key should be string since JSON map keys are always strings
        let map_obj = json["data"].as_object().unwrap();
        assert!(map_obj.contains_key("18446744073709551615"));
        let restored: TestNestedMap = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn nested_map_inner_small_values_stay_numbers() {
        let id = Identifier::random();
        let mut inner = BTreeMap::new();
        inner.insert(id, 42u64);
        let mut data = BTreeMap::new();
        data.insert(1u64, inner);
        let t = TestNestedMap { data };
        let json = serde_json::to_value(&t).unwrap();

        // Navigate to the inner map value and verify it's a number
        let outer = json["data"].as_object().unwrap();
        let inner_map = outer["1"].as_object().unwrap();
        let val = inner_map.values().next().unwrap();
        assert!(val.is_number());
    }

    // --- Additional tests for json_safe_generic_u64_value_map ---

    #[test]
    fn generic_map_empty_round_trip() {
        let t = TestGenericMap {
            data: BTreeMap::new(),
        };
        let json = serde_json::to_value(&t).unwrap();
        let restored: TestGenericMap = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn generic_map_platform_value_empty_round_trip() {
        let t = TestGenericMap {
            data: BTreeMap::new(),
        };
        let pv = platform_value::to_value(&t).unwrap();
        let restored: TestGenericMap = platform_value::from_value(pv).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn generic_map_at_safe_boundary_stays_number() {
        let mut data = BTreeMap::new();
        data.insert(CustomKey("boundary".into()), JS_MAX_SAFE_INTEGER);
        let t = TestGenericMap { data };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["data"]["boundary"].is_number());

        let restored: TestGenericMap = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn generic_map_above_safe_boundary_becomes_string() {
        let mut data = BTreeMap::new();
        data.insert(CustomKey("above".into()), JS_MAX_SAFE_INTEGER + 1);
        let t = TestGenericMap { data };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["data"]["above"].is_string());

        let restored: TestGenericMap = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn generic_map_zero_value_round_trip() {
        let mut data = BTreeMap::new();
        data.insert(CustomKey("zero".into()), 0u64);
        let t = TestGenericMap { data };
        let json = serde_json::to_value(&t).unwrap();
        assert!(json["data"]["zero"].is_number());
        assert_eq!(json["data"]["zero"].as_u64().unwrap(), 0);

        let restored: TestGenericMap = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    // --- Error path tests for generic_map ---

    #[test]
    fn generic_map_invalid_value_type_fails() {
        let json = serde_json::json!({"data": {"key": true}});
        let result = serde_json::from_value::<TestGenericMap>(json);
        assert!(result.is_err());
    }

    #[test]
    fn generic_map_invalid_value_string_fails() {
        let json = serde_json::json!({"data": {"key": "not_a_number"}});
        let result = serde_json::from_value::<TestGenericMap>(json);
        assert!(result.is_err());
    }

    #[test]
    fn generic_map_null_value_fails() {
        let json = serde_json::json!({"data": {"key": null}});
        let result = serde_json::from_value::<TestGenericMap>(json);
        assert!(result.is_err());
    }

    #[test]
    fn generic_map_array_value_fails() {
        let json = serde_json::json!({"data": {"key": [1, 2, 3]}});
        let result = serde_json::from_value::<TestGenericMap>(json);
        assert!(result.is_err());
    }

    // --- Error path tests for identifier_u64_map ---

    #[test]
    fn identifier_u64_map_invalid_value_string_fails() {
        let id = Identifier::random();
        let json = serde_json::json!({"data": {id.to_string(Encoding::Base58): "not_a_number"}});
        let result = serde_json::from_value::<TestIdentifierU64Map>(json);
        assert!(result.is_err());
    }

    #[test]
    fn identifier_u64_map_null_value_fails() {
        let id = Identifier::random();
        let json = serde_json::json!({"data": {id.to_string(Encoding::Base58): null}});
        let result = serde_json::from_value::<TestIdentifierU64Map>(json);
        assert!(result.is_err());
    }

    // --- Error path tests for nested_map ---

    #[test]
    fn nested_map_invalid_inner_value_type_fails() {
        let id = Identifier::random();
        let json =
            serde_json::json!({"data": {"1": {id.to_string(Encoding::Base58): "not_a_number"}}});
        let result = serde_json::from_value::<TestNestedMap>(json);
        assert!(result.is_err());
    }

    #[test]
    fn nested_map_invalid_outer_key_fails() {
        let id = Identifier::random();
        let json =
            serde_json::json!({"data": {"not_a_number": {id.to_string(Encoding::Base58): 42}}});
        let result = serde_json::from_value::<TestNestedMap>(json);
        assert!(result.is_err());
    }

    // --- u64_u64_map additional tests ---

    #[test]
    fn u64_u64_map_at_safe_boundary_stays_number() {
        let mut data = BTreeMap::new();
        data.insert(1u64, JS_MAX_SAFE_INTEGER);
        let t = TestU64U64Map { data };
        let json = serde_json::to_value(&t).unwrap();
        let map_obj = json["data"].as_object().unwrap();
        let val = &map_obj["1"];
        assert!(val.is_number());

        let restored: TestU64U64Map = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn u64_u64_map_above_safe_boundary_becomes_string() {
        let mut data = BTreeMap::new();
        data.insert(1u64, JS_MAX_SAFE_INTEGER + 1);
        let t = TestU64U64Map { data };
        let json = serde_json::to_value(&t).unwrap();
        let map_obj = json["data"].as_object().unwrap();
        let val = &map_obj["1"];
        assert!(val.is_string());

        let restored: TestU64U64Map = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn u64_u64_map_multiple_entries_round_trip() {
        let mut data = BTreeMap::new();
        data.insert(0u64, 0u64);
        data.insert(42u64, JS_MAX_SAFE_INTEGER);
        data.insert(u64::MAX, u64::MAX);
        let t = TestU64U64Map { data };
        let json = serde_json::to_value(&t).unwrap();

        // Value 0 should be a number
        let map_obj = json["data"].as_object().unwrap();
        assert!(map_obj["0"].is_number());
        // JS_MAX_SAFE_INTEGER should be a number
        assert!(map_obj["42"].is_number());
        // u64::MAX should be a string
        assert!(map_obj["18446744073709551615"].is_string());

        let restored: TestU64U64Map = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn identifier_u64_map_multiple_entries_round_trip() {
        let id1 = Identifier::random();
        let id2 = Identifier::random();
        let mut data = BTreeMap::new();
        data.insert(id1, 0u64);
        data.insert(id2, u64::MAX);
        let t = TestIdentifierU64Map { data };
        let json = serde_json::to_value(&t).unwrap();
        let restored: TestIdentifierU64Map = serde_json::from_value(json).unwrap();
        assert_eq!(t, restored);
    }
}
