use crate::identity::TimestampMillis;
use crate::prelude::Revision;
use platform_value::{Identifier, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Documents contain the data that goes into data contracts.
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

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
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

    fn assert_fields(p: &DocumentPatch) {
        assert_eq!(p.id, Identifier::new([0x77; 32]), "id");
        assert_eq!(p.properties.len(), 2, "properties count");
        assert_eq!(p.revision, Some(3), "revision");
        assert_eq!(p.updated_at, Some(1_700_000_000_000), "updated_at");
    }

    #[test]
    fn json_round_trip_with_per_property_assertions() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered = DocumentPatch::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
        assert_fields(&recovered);
    }

    #[test]
    fn value_round_trip_with_per_property_assertions() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered = DocumentPatch::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
        assert_fields(&recovered);
    }
}
