use crate::identity::TimestampMillis;
use crate::prelude::Revision;
use platform_value::{Identifier, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Documents contain the data that goes into data contracts.
// Auto-injects `json_safe_option_u64` on `revision: Option<Revision>` and
// `updated_at: Option<TimestampMillis>` (both Option<u64>). The `properties:
// BTreeMap<String, Value>` flatten catchall is skipped by the macro.
#[cfg_attr(feature = "json-conversion", crate::serialization::json_safe_fields)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct DocumentPatch {
    /// The unique document ID.
    #[serde(rename = "$id")]
    pub id: Identifier,
    /// The document's properties (data).
    #[serde(flatten)]
    pub properties: BTreeMap<String, Value>,
    /// The document revision.
    #[serde(rename = "$revision")]
    pub revision: Option<Revision>,
    #[serde(rename = "$updatedAt")]
    pub updated_at: Option<TimestampMillis>,
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for DocumentPatch {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for DocumentPatch {}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests_documentpatch {
    use super::*;
    use platform_value::Identifier;

    fn fixture() -> DocumentPatch {
        let mut properties = BTreeMap::new();
        properties.insert("name".to_string(), Value::Text("alice".to_string()));
        properties.insert("count".to_string(), Value::U64(42));
        DocumentPatch {
            id: Identifier::new([0x77; 32]),
            properties,
            revision: Some(3),
            updated_at: Some(1_700_000_000_000),
        }
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        use serde_json::json;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `DocumentPatch` uses `#[serde(flatten)]` for `properties`, so the
        // `name` / `count` keys are inlined at the top level. `Identifier` is
        // base58 in JSON. `revision` is `Option<Revision>` (u64) and
        // `updated_at` is `Option<TimestampMillis>` (u64); JSON erases the
        // u64 distinction (value-path locks `3u64` / `1_700_000_000_000_u64`).
        assert_eq!(
            json,
            json!({
                "$id": "93MB2qRDNVLxbmmPuYpLdAqn3u2x9ZhaVZK5wELHueP8",
                "count": 42,
                "name": "alice",
                "$revision": 3,
                "$updatedAt": 1_700_000_000_000_u64,
            })
        );
        let recovered = DocumentPatch::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        use platform_value::platform_value;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // Non-HR: `$id` stays `Value::Identifier`; `count` was stored as
        // `Value::U64(42)` directly in the fixture; revision/updated_at are u64.
        assert_eq!(
            value,
            platform_value!({
                "$id": Identifier::new([0x77; 32]),
                "count": 42u64,
                "name": "alice",
                "$revision": 3u64,
                "$updatedAt": 1_700_000_000_000_u64,
            })
        );
        let recovered = DocumentPatch::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
