use crate::document::serialization_traits::DocumentPlatformValueMethodsV0;
use crate::document::DocumentV0;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;
use std::collections::BTreeMap;

impl DocumentPlatformValueMethodsV0<'_> for DocumentV0 {
    fn to_map_value(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        Ok(platform_value::to_value(self)?.into_btree_string_map()?)
    }

    fn into_map_value(self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        Ok(platform_value::to_value(self)?.into_btree_string_map()?)
    }

    fn into_value(self) -> Result<Value, ProtocolError> {
        Ok(platform_value::to_value(self)?)
    }

    fn to_object(&self) -> Result<Value, ProtocolError> {
        Ok(platform_value::to_value(self)?)
    }

    fn from_platform_value(
        document_value: Value,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        Ok(platform_value::from_value(document_value)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::property_names;
    use platform_value::Identifier;
    use platform_version::version::PlatformVersion;

    fn minimal_doc() -> DocumentV0 {
        DocumentV0 {
            id: Identifier::new([1u8; 32]),
            owner_id: Identifier::new([2u8; 32]),
            properties: BTreeMap::new(),
            revision: None,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        }
    }

    fn full_doc() -> DocumentV0 {
        let mut props = BTreeMap::new();
        props.insert("name".into(), Value::Text("Eve".into()));
        props.insert("score".into(), Value::U64(42));
        DocumentV0 {
            id: Identifier::new([7u8; 32]),
            owner_id: Identifier::new([8u8; 32]),
            properties: props,
            revision: Some(3),
            created_at: Some(1_700_000_000_000),
            updated_at: Some(1_700_000_100_000),
            transferred_at: Some(1_700_000_200_000),
            created_at_block_height: Some(10),
            updated_at_block_height: Some(20),
            transferred_at_block_height: Some(30),
            created_at_core_block_height: Some(1),
            updated_at_core_block_height: Some(2),
            transferred_at_core_block_height: Some(3),
            creator_id: Some(Identifier::new([9u8; 32])),
        }
    }

    // ================================================================
    //  to_map_value: produces a BTreeMap<String, Value> keyed by the
    //  documented serde field names (with the $-prefixed renames).
    // ================================================================

    #[test]
    fn to_map_value_contains_id_and_owner_id_keys() {
        let doc = minimal_doc();
        let map = doc.to_map_value().expect("to_map_value should succeed");
        assert!(map.contains_key(property_names::ID));
        assert!(map.contains_key(property_names::OWNER_ID));
    }

    #[test]
    fn to_map_value_contains_all_set_optional_fields() {
        let doc = full_doc();
        let map = doc.to_map_value().expect("to_map_value should succeed");
        assert!(map.contains_key(property_names::REVISION));
        assert!(map.contains_key(property_names::CREATED_AT));
        assert!(map.contains_key(property_names::UPDATED_AT));
        assert!(map.contains_key(property_names::TRANSFERRED_AT));
        assert!(map.contains_key(property_names::CREATED_AT_BLOCK_HEIGHT));
        assert!(map.contains_key(property_names::UPDATED_AT_BLOCK_HEIGHT));
        assert!(map.contains_key(property_names::TRANSFERRED_AT_BLOCK_HEIGHT));
        assert!(map.contains_key(property_names::CREATED_AT_CORE_BLOCK_HEIGHT));
        assert!(map.contains_key(property_names::UPDATED_AT_CORE_BLOCK_HEIGHT));
        assert!(map.contains_key(property_names::TRANSFERRED_AT_CORE_BLOCK_HEIGHT));
        assert!(map.contains_key(property_names::CREATOR_ID));
        // User-defined properties are flattened into the map
        assert!(map.contains_key("name"));
        assert!(map.contains_key("score"));
    }

    // ================================================================
    //  into_map_value: same shape as to_map_value, consumes self
    // ================================================================

    #[test]
    fn into_map_value_consumes_and_returns_same_shape_as_to_map_value() {
        let doc = full_doc();
        let from_ref = doc.to_map_value().expect("to_map_value");
        let from_owned = doc.into_map_value().expect("into_map_value");
        assert_eq!(from_ref, from_owned);
    }

    // ================================================================
    //  to_object / into_value: produce a Value::Map
    // ================================================================

    #[test]
    fn to_object_returns_a_map_value() {
        let doc = full_doc();
        let v = doc.to_object().expect("to_object");
        assert!(v.is_map(), "Expected a Value::Map, got {:?}", v);
    }

    #[test]
    fn into_value_consumes_and_returns_a_map_value() {
        let doc = full_doc();
        let v = doc.into_value().expect("into_value");
        assert!(v.is_map(), "Expected a Value::Map, got {:?}", v);
    }

    // ================================================================
    //  from_platform_value round-trip: to_object -> from_platform_value
    // ================================================================

    #[test]
    fn from_platform_value_round_trip_preserves_all_fields() {
        let platform_version = PlatformVersion::latest();
        let doc = full_doc();
        let v = doc.to_object().expect("to_object");
        let recovered = DocumentV0::from_platform_value(v, platform_version)
            .expect("from_platform_value should succeed");
        assert_eq!(doc, recovered);
    }

    #[test]
    fn from_platform_value_round_trip_with_minimal_fields() {
        let platform_version = PlatformVersion::latest();
        let doc = minimal_doc();
        let v = doc.to_object().expect("to_object");
        let recovered = DocumentV0::from_platform_value(v, platform_version)
            .expect("from_platform_value should succeed");
        assert_eq!(doc, recovered);
    }

    // ================================================================
    //  from_platform_value error path: non-map Value should fail
    // ================================================================

    #[test]
    fn from_platform_value_with_non_map_value_returns_error() {
        let platform_version = PlatformVersion::latest();
        let bad = Value::Text("not a document".to_string());
        let result = DocumentV0::from_platform_value(bad, platform_version);
        assert!(
            result.is_err(),
            "from_platform_value with a non-map Value should fail"
        );
    }
}
