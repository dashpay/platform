use crate::document::fields::property_names;
use crate::document::serialization_traits::DocumentJsonMethodsV0;
use crate::document::DocumentV0;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use serde_json::{json, Value as JsonValue};

impl DocumentJsonMethodsV0<'_> for DocumentV0 {
    fn to_json_with_identifiers_using_bytes(
        &self,
        _platform_version: &PlatformVersion,
    ) -> Result<JsonValue, ProtocolError> {
        let mut value = json!({
            property_names::ID: self.id,
            property_names::OWNER_ID: self.owner_id,
        });
        let value_mut = value.as_object_mut().unwrap();
        if let Some(created_at) = self.created_at {
            value_mut.insert(
                property_names::CREATED_AT.to_string(),
                JsonValue::Number(created_at.into()),
            );
        }
        if let Some(updated_at) = self.updated_at {
            value_mut.insert(
                property_names::UPDATED_AT.to_string(),
                JsonValue::Number(updated_at.into()),
            );
        }
        if let Some(created_at_block_height) = self.created_at_block_height {
            value_mut.insert(
                property_names::CREATED_AT_BLOCK_HEIGHT.to_string(),
                JsonValue::Number(created_at_block_height.into()),
            );
        }

        if let Some(updated_at_block_height) = self.updated_at_block_height {
            value_mut.insert(
                property_names::UPDATED_AT_BLOCK_HEIGHT.to_string(),
                JsonValue::Number(updated_at_block_height.into()),
            );
        }

        if let Some(created_at_core_block_height) = self.created_at_core_block_height {
            value_mut.insert(
                property_names::CREATED_AT_CORE_BLOCK_HEIGHT.to_string(),
                JsonValue::Number(created_at_core_block_height.into()),
            );
        }

        if let Some(updated_at_core_block_height) = self.updated_at_core_block_height {
            value_mut.insert(
                property_names::UPDATED_AT_CORE_BLOCK_HEIGHT.to_string(),
                JsonValue::Number(updated_at_core_block_height.into()),
            );
        }
        if let Some(transferred_at) = self.transferred_at {
            value_mut.insert(
                property_names::TRANSFERRED_AT.to_string(),
                JsonValue::Number(transferred_at.into()),
            );
        }
        if let Some(transferred_at_block_height) = self.transferred_at_block_height {
            value_mut.insert(
                property_names::TRANSFERRED_AT_BLOCK_HEIGHT.to_string(),
                JsonValue::Number(transferred_at_block_height.into()),
            );
        }
        if let Some(transferred_at_core_block_height) = self.transferred_at_core_block_height {
            value_mut.insert(
                property_names::TRANSFERRED_AT_CORE_BLOCK_HEIGHT.to_string(),
                JsonValue::Number(transferred_at_core_block_height.into()),
            );
        }
        if let Some(creator_id) = self.creator_id {
            value_mut.insert(property_names::CREATOR_ID.to_string(), json!(creator_id));
        }
        if let Some(revision) = self.revision {
            value_mut.insert(
                property_names::REVISION.to_string(),
                JsonValue::Number(revision.into()),
            );
        }

        self.properties
            .iter()
            .try_for_each(|(key, property_value)| {
                let serde_value: JsonValue = property_value.try_to_validating_json()?;
                value_mut.insert(key.to_string(), serde_value);
                Ok::<(), ProtocolError>(())
            })?;

        Ok(value)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::serialization_traits::DocumentJsonMethodsV0;
    use platform_value::{Identifier, Value};
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn make_document_v0_with_all_timestamps() -> DocumentV0 {
        let mut properties = BTreeMap::new();
        properties.insert("label".to_string(), Value::Text("test-label".to_string()));
        DocumentV0 {
            id: Identifier::new([1u8; 32]),
            owner_id: Identifier::new([2u8; 32]),
            properties,
            revision: Some(3),
            created_at: Some(1_700_000_000_000),
            updated_at: Some(1_700_000_100_000),
            transferred_at: Some(1_700_000_200_000),
            created_at_block_height: Some(100),
            updated_at_block_height: Some(200),
            transferred_at_block_height: Some(300),
            created_at_core_block_height: Some(50),
            updated_at_core_block_height: Some(60),
            transferred_at_core_block_height: Some(70),
            creator_id: Some(Identifier::new([9u8; 32])),
        }
    }

    fn make_minimal_document_v0() -> DocumentV0 {
        DocumentV0 {
            id: Identifier::new([0xAA; 32]),
            owner_id: Identifier::new([0xBB; 32]),
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

    // ================================================================
    //  to_json produces a JsonValue containing all set fields
    // ================================================================

    #[test]
    fn to_json_includes_id_and_owner_id() {
        // DocumentV0 doesn't derive `JsonConvertible` (only the outer
        // `Document` enum does). Tests use `serde_json::to_value` directly
        // for the canonical serde shape.
        let doc = make_minimal_document_v0();
        let json: JsonValue = serde_json::to_value(&doc).expect("to_value should succeed");
        let obj = json.as_object().expect("should be an object");
        assert!(
            obj.contains_key(property_names::ID),
            "JSON should contain $id"
        );
        assert!(
            obj.contains_key(property_names::OWNER_ID),
            "JSON should contain $ownerId"
        );
    }

    #[test]
    fn to_json_represents_none_timestamps_as_null() {
        let doc = make_minimal_document_v0();
        let json: JsonValue = serde_json::to_value(&doc).expect("to_value should succeed");
        let obj = json.as_object().expect("should be an object");

        // to_json serializes via serde, so None fields appear as null
        if let Some(val) = obj.get(property_names::CREATED_AT) {
            assert!(
                val.is_null(),
                "$createdAt should be null when None, got: {:?}",
                val
            );
        }
        if let Some(val) = obj.get(property_names::UPDATED_AT) {
            assert!(
                val.is_null(),
                "$updatedAt should be null when None, got: {:?}",
                val
            );
        }
        if let Some(val) = obj.get(property_names::REVISION) {
            assert!(
                val.is_null(),
                "$revision should be null when None, got: {:?}",
                val
            );
        }
    }

    // ================================================================
    //  to_json_with_identifiers_using_bytes includes all timestamps
    // ================================================================

    #[test]
    fn to_json_with_identifiers_using_bytes_includes_all_timestamp_fields() {
        let platform_version = PlatformVersion::latest();
        let doc = make_document_v0_with_all_timestamps();
        let json = doc
            .to_json_with_identifiers_using_bytes(platform_version)
            .expect("to_json_with_identifiers_using_bytes should succeed");
        let obj = json.as_object().expect("should be an object");

        assert!(obj.contains_key(property_names::ID));
        assert!(obj.contains_key(property_names::OWNER_ID));
        assert!(obj.contains_key(property_names::REVISION));
        assert!(obj.contains_key(property_names::CREATED_AT));
        assert!(obj.contains_key(property_names::UPDATED_AT));
        assert!(obj.contains_key(property_names::TRANSFERRED_AT));
        assert!(obj.contains_key(property_names::CREATED_AT_BLOCK_HEIGHT));
        assert!(obj.contains_key(property_names::UPDATED_AT_BLOCK_HEIGHT));
        assert!(obj.contains_key(property_names::TRANSFERRED_AT_BLOCK_HEIGHT));
        assert!(obj.contains_key(property_names::CREATED_AT_CORE_BLOCK_HEIGHT));
        assert!(obj.contains_key(property_names::UPDATED_AT_CORE_BLOCK_HEIGHT));
        assert!(obj.contains_key(property_names::TRANSFERRED_AT_CORE_BLOCK_HEIGHT));
        assert!(obj.contains_key(property_names::CREATOR_ID));

        // Verify numeric values
        assert_eq!(obj[property_names::REVISION].as_u64(), Some(3));
        assert_eq!(
            obj[property_names::CREATED_AT].as_u64(),
            Some(1_700_000_000_000)
        );
        assert_eq!(
            obj[property_names::UPDATED_AT].as_u64(),
            Some(1_700_000_100_000)
        );
        assert_eq!(
            obj[property_names::TRANSFERRED_AT].as_u64(),
            Some(1_700_000_200_000)
        );
        assert_eq!(
            obj[property_names::CREATED_AT_BLOCK_HEIGHT].as_u64(),
            Some(100)
        );
        assert_eq!(
            obj[property_names::UPDATED_AT_BLOCK_HEIGHT].as_u64(),
            Some(200)
        );
        assert_eq!(
            obj[property_names::TRANSFERRED_AT_BLOCK_HEIGHT].as_u64(),
            Some(300)
        );
        assert_eq!(
            obj[property_names::CREATED_AT_CORE_BLOCK_HEIGHT].as_u64(),
            Some(50)
        );
        assert_eq!(
            obj[property_names::UPDATED_AT_CORE_BLOCK_HEIGHT].as_u64(),
            Some(60)
        );
        assert_eq!(
            obj[property_names::TRANSFERRED_AT_CORE_BLOCK_HEIGHT].as_u64(),
            Some(70)
        );
    }

    #[test]
    fn to_json_with_identifiers_using_bytes_includes_custom_properties() {
        let platform_version = PlatformVersion::latest();
        let doc = make_document_v0_with_all_timestamps();
        let json = doc
            .to_json_with_identifiers_using_bytes(platform_version)
            .expect("should succeed");
        let obj = json.as_object().expect("should be an object");
        assert_eq!(
            obj.get("label").and_then(|v| v.as_str()),
            Some("test-label")
        );
    }

    // ================================================================
    //  Note: legacy `from_json_value` ingest tests were removed in
    //  Phase D step 8 slice B alongside the deleted method itself. The
    //  canonical JSON round-trip is exercised in
    //  `document/v0/serialize.rs` and at the outer-Document level via
    //  `JsonConvertible` (see `serialization::value_convertible` and
    //  the `Document` impl in `serialization::json_convertible`).
    // ================================================================

    // ================================================================
    //  to_json_with_identifiers_using_bytes: minimal document has only
    //  $id and $ownerId keys (no optional fields rendered).
    // ================================================================

    #[test]
    fn to_json_with_identifiers_using_bytes_minimal_document_has_only_id_and_owner() {
        let platform_version = PlatformVersion::latest();
        let doc = make_minimal_document_v0();
        let json = doc
            .to_json_with_identifiers_using_bytes(platform_version)
            .expect("to_json_with_identifiers_using_bytes should succeed");
        let obj = json.as_object().expect("object");
        assert!(obj.contains_key(property_names::ID));
        assert!(obj.contains_key(property_names::OWNER_ID));
        // None-valued optional fields are NOT emitted by this serializer.
        assert!(!obj.contains_key(property_names::CREATED_AT));
        assert!(!obj.contains_key(property_names::UPDATED_AT));
        assert!(!obj.contains_key(property_names::TRANSFERRED_AT));
        assert!(!obj.contains_key(property_names::CREATED_AT_BLOCK_HEIGHT));
        assert!(!obj.contains_key(property_names::UPDATED_AT_BLOCK_HEIGHT));
        assert!(!obj.contains_key(property_names::TRANSFERRED_AT_BLOCK_HEIGHT));
        assert!(!obj.contains_key(property_names::CREATED_AT_CORE_BLOCK_HEIGHT));
        assert!(!obj.contains_key(property_names::UPDATED_AT_CORE_BLOCK_HEIGHT));
        assert!(!obj.contains_key(property_names::TRANSFERRED_AT_CORE_BLOCK_HEIGHT));
        assert!(!obj.contains_key(property_names::CREATOR_ID));
        assert!(!obj.contains_key(property_names::REVISION));
    }

    // ================================================================
    //  to_json_with_identifiers_using_bytes: id/owner emitted as
    //  base58 strings (via serde_json derive on Identifier).
    // ================================================================

    #[test]
    fn to_json_with_identifiers_using_bytes_emits_base58_identifiers() {
        let platform_version = PlatformVersion::latest();
        let doc = make_minimal_document_v0();
        let json = doc
            .to_json_with_identifiers_using_bytes(platform_version)
            .expect("should succeed");
        let obj = json.as_object().expect("object");
        // $id and $ownerId are serialized as base58 strings by Identifier's
        // Serialize impl, which is what the underlying json! macro uses.
        let id_val = obj.get(property_names::ID).expect("id present");
        assert!(id_val.is_string(), "expected base58 string for $id");
        let owner_val = obj.get(property_names::OWNER_ID).expect("owner present");
        assert!(owner_val.is_string(), "expected base58 string for $ownerId");
    }

    // ================================================================
    //  to_json_with_identifiers_using_bytes: multiple user-defined
    //  properties are all included.
    // ================================================================

    #[test]
    fn to_json_with_identifiers_using_bytes_with_multiple_properties() {
        let platform_version = PlatformVersion::latest();
        let mut props = BTreeMap::new();
        props.insert("a".to_string(), Value::U64(1));
        props.insert("b".to_string(), Value::Text("two".to_string()));
        props.insert("c".to_string(), Value::Bool(true));
        let doc = DocumentV0 {
            id: Identifier::new([1u8; 32]),
            owner_id: Identifier::new([2u8; 32]),
            properties: props,
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
        };
        let json = doc
            .to_json_with_identifiers_using_bytes(platform_version)
            .expect("should succeed");
        let obj = json.as_object().expect("object");
        assert_eq!(obj.get("a").and_then(|v| v.as_u64()), Some(1));
        assert_eq!(obj.get("b").and_then(|v| v.as_str()), Some("two"));
        assert_eq!(obj.get("c").and_then(|v| v.as_bool()), Some(true));
    }

}
