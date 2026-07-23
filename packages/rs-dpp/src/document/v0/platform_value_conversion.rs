use crate::document::serialization_traits::DocumentPlatformValueMethodsV0;
use crate::document::DocumentV0;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::property_names;
    use platform_value::Identifier;

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

    // After Phase D step 8 slice A, the `to_object` / `into_value` /
    // `from_platform_value` tests on this V0 inner moved to the outer
    // `Document` enum's canonical-trait round-trip tests in
    // `serialization_traits/platform_value_conversion/mod.rs`. The
    // `to_map_value` / `into_map_value` tests stay because those are
    // the methods this trait still defines.
}
