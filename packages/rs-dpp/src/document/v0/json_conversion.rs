use crate::document::fields::property_names;
use crate::document::serialization_traits::{
    DocumentJsonMethodsV0, DocumentPlatformValueMethodsV0,
};
use crate::document::DocumentV0;
use crate::util::json_value::JsonValueExt;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
use serde::Deserialize;
use serde_json::{json, Value as JsonValue};
use std::convert::TryInto;

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

    fn to_json(&self, _platform_version: &PlatformVersion) -> Result<JsonValue, ProtocolError> {
        self.to_object()
            .map(|v| v.try_into().map_err(ProtocolError::ValueError))?
    }

    fn from_json_value<S, E>(
        mut document_value: JsonValue,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        for<'de> S: Deserialize<'de> + TryInto<Identifier, Error = E>,
        E: Into<ProtocolError>,
    {
        let mut document = Self {
            ..Default::default()
        };

        if let Ok(value) = document_value.remove(property_names::ID) {
            if !value.is_null() {
                let data: S = serde_json::from_value(value)?;
                document.id = data.try_into().map_err(Into::into)?;
            }
        }
        if let Ok(value) = document_value.remove(property_names::OWNER_ID) {
            if !value.is_null() {
                let data: S = serde_json::from_value(value)?;
                document.owner_id = data.try_into().map_err(Into::into)?;
            }
        }
        if let Ok(value) = document_value.remove(property_names::REVISION) {
            document.revision = serde_json::from_value(value)?
        }
        if let Ok(value) = document_value.remove(property_names::CREATED_AT) {
            document.created_at = serde_json::from_value(value)?
        }
        if let Ok(value) = document_value.remove(property_names::UPDATED_AT) {
            document.updated_at = serde_json::from_value(value)?
        }
        if let Ok(value) = document_value.remove(property_names::CREATED_AT_BLOCK_HEIGHT) {
            document.created_at_block_height = serde_json::from_value(value)?;
        }
        if let Ok(value) = document_value.remove(property_names::UPDATED_AT_BLOCK_HEIGHT) {
            document.updated_at_block_height = serde_json::from_value(value)?;
        }
        if let Ok(value) = document_value.remove(property_names::CREATED_AT_CORE_BLOCK_HEIGHT) {
            document.created_at_core_block_height = serde_json::from_value(value)?;
        }
        if let Ok(value) = document_value.remove(property_names::UPDATED_AT_CORE_BLOCK_HEIGHT) {
            document.updated_at_core_block_height = serde_json::from_value(value)?;
        }
        if let Ok(value) = document_value.remove(property_names::TRANSFERRED_AT) {
            document.transferred_at = serde_json::from_value(value)?;
        }
        if let Ok(value) = document_value.remove(property_names::TRANSFERRED_AT_BLOCK_HEIGHT) {
            document.transferred_at_block_height = serde_json::from_value(value)?;
        }
        if let Ok(value) = document_value.remove(property_names::TRANSFERRED_AT_CORE_BLOCK_HEIGHT) {
            document.transferred_at_core_block_height = serde_json::from_value(value)?;
        }
        if let Ok(value) = document_value.remove(property_names::CREATOR_ID) {
            if !value.is_null() {
                let data: S = serde_json::from_value(value)?;
                document.creator_id = Some(data.try_into().map_err(Into::into)?);
            }
        }

        let platform_value: Value = document_value.into();

        document.properties = platform_value
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;
        Ok(document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::document_type::random_document::CreateRandomDocument;
    use crate::document::serialization_traits::DocumentJsonMethodsV0;
    use crate::tests::json_document::json_document_to_contract;
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
        let platform_version = PlatformVersion::latest();
        let doc = make_minimal_document_v0();
        let json = doc
            .to_json(platform_version)
            .expect("to_json should succeed");
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
        let platform_version = PlatformVersion::latest();
        let doc = make_minimal_document_v0();
        let json = doc
            .to_json(platform_version)
            .expect("to_json should succeed");
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
    //  from_json_value round-trip: to_json -> from_json_value
    //  Uses String as the identifier deserialization type since
    //  to_json produces base58 string identifiers.
    // ================================================================

    #[test]
    fn json_round_trip_with_random_dashpay_profile() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected to load dashpay contract");

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected profile document type");

        for seed in 0..5u64 {
            let document = document_type
                .random_document(Some(seed), platform_version)
                .expect("expected random document");

            let crate::document::Document::V0(doc_v0) = &document;

            let json_val = doc_v0
                .to_json(platform_version)
                .expect("to_json should succeed");

            let recovered = DocumentV0::from_json_value::<String, _>(json_val, platform_version)
                .expect("from_json_value should succeed");

            assert_eq!(doc_v0.id, recovered.id, "id mismatch for seed {seed}");
            assert_eq!(
                doc_v0.owner_id, recovered.owner_id,
                "owner_id mismatch for seed {seed}"
            );
            assert_eq!(
                doc_v0.revision, recovered.revision,
                "revision mismatch for seed {seed}"
            );
        }
    }

    // ================================================================
    //  from_json_value extracts all system fields correctly
    // ================================================================

    #[test]
    fn from_json_value_extracts_timestamps_and_revision() {
        let platform_version = PlatformVersion::latest();
        let id = Identifier::new([1u8; 32]);
        let owner = Identifier::new([2u8; 32]);
        let creator = Identifier::new([9u8; 32]);

        let json_val = json!({
            "$id": bs58::encode(id.to_buffer()).into_string(),
            "$ownerId": bs58::encode(owner.to_buffer()).into_string(),
            "$revision": 5,
            "$createdAt": 1_000_000u64,
            "$updatedAt": 2_000_000u64,
            "$createdAtBlockHeight": 100u64,
            "$updatedAtBlockHeight": 200u64,
            "$createdAtCoreBlockHeight": 50u32,
            "$updatedAtCoreBlockHeight": 60u32,
            "$transferredAt": 3_000_000u64,
            "$transferredAtBlockHeight": 300u64,
            "$transferredAtCoreBlockHeight": 70u32,
            "$creatorId": bs58::encode(creator.to_buffer()).into_string(),
            "customProp": "hello"
        });

        let doc = DocumentV0::from_json_value::<String, _>(json_val, platform_version)
            .expect("from_json_value should succeed");

        assert_eq!(doc.id, id);
        assert_eq!(doc.owner_id, owner);
        assert_eq!(doc.revision, Some(5));
        assert_eq!(doc.created_at, Some(1_000_000));
        assert_eq!(doc.updated_at, Some(2_000_000));
        assert_eq!(doc.created_at_block_height, Some(100));
        assert_eq!(doc.updated_at_block_height, Some(200));
        assert_eq!(doc.created_at_core_block_height, Some(50));
        assert_eq!(doc.updated_at_core_block_height, Some(60));
        assert_eq!(doc.transferred_at, Some(3_000_000));
        assert_eq!(doc.transferred_at_block_height, Some(300));
        assert_eq!(doc.transferred_at_core_block_height, Some(70));
        assert_eq!(doc.creator_id, Some(creator));
        // Custom property should be in properties map
        assert_eq!(
            doc.properties.get("customProp"),
            Some(&Value::Text("hello".to_string()))
        );
    }

    #[test]
    fn from_json_value_handles_missing_optional_fields() {
        let platform_version = PlatformVersion::latest();
        let id = Identifier::new([3u8; 32]);
        let owner = Identifier::new([4u8; 32]);
        let json_val = json!({
            "$id": bs58::encode(id.to_buffer()).into_string(),
            "$ownerId": bs58::encode(owner.to_buffer()).into_string(),
        });

        let doc = DocumentV0::from_json_value::<String, _>(json_val, platform_version)
            .expect("from_json_value should succeed with minimal fields");

        assert_eq!(doc.id, id);
        assert_eq!(doc.owner_id, owner);
        assert_eq!(doc.revision, None);
        assert_eq!(doc.created_at, None);
        assert_eq!(doc.updated_at, None);
        assert_eq!(doc.transferred_at, None);
        assert_eq!(doc.created_at_block_height, None);
        assert_eq!(doc.updated_at_block_height, None);
        assert_eq!(doc.transferred_at_block_height, None);
        assert_eq!(doc.created_at_core_block_height, None);
        assert_eq!(doc.updated_at_core_block_height, None);
        assert_eq!(doc.transferred_at_core_block_height, None);
        assert_eq!(doc.creator_id, None);
    }

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
    //  from_json_value handles null creator_id by leaving it None.
    // ================================================================

    #[test]
    fn from_json_value_with_null_creator_id_stays_none() {
        let platform_version = PlatformVersion::latest();
        let json_val = json!({
            "$id": bs58::encode([1u8; 32]).into_string(),
            "$ownerId": bs58::encode([2u8; 32]).into_string(),
            "$creatorId": JsonValue::Null,
        });
        let doc = DocumentV0::from_json_value::<String, _>(json_val, platform_version)
            .expect("from_json_value should succeed with null creator_id");
        assert_eq!(doc.creator_id, None);
    }

    // ================================================================
    //  from_json_value handles null id/owner by leaving them defaulted.
    // ================================================================

    #[test]
    fn from_json_value_with_null_id_leaves_default() {
        let platform_version = PlatformVersion::latest();
        let json_val = json!({
            "$id": JsonValue::Null,
            "$ownerId": bs58::encode([2u8; 32]).into_string(),
        });
        let doc = DocumentV0::from_json_value::<String, _>(json_val, platform_version)
            .expect("from_json_value should succeed with null $id");
        // Default Identifier is all-zeros.
        assert_eq!(doc.id, Identifier::new([0u8; 32]));
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

    // ================================================================
    //  from_json_value: an empty object produces a fully-defaulted doc
    // ================================================================

    #[test]
    fn from_json_value_empty_object_returns_default_document() {
        let platform_version = PlatformVersion::latest();
        let doc = DocumentV0::from_json_value::<String, _>(json!({}), platform_version)
            .expect("from_json_value should succeed with empty object");
        assert_eq!(doc.id, Identifier::new([0u8; 32]));
        assert_eq!(doc.owner_id, Identifier::new([0u8; 32]));
        assert_eq!(doc.revision, None);
        assert!(doc.properties.is_empty());
    }

    // ================================================================
    //  from_json_value with creator_id
    // ================================================================

    #[test]
    fn from_json_value_parses_creator_id() {
        let platform_version = PlatformVersion::latest();
        let creator = Identifier::new([0xCC; 32]);
        let json_val = json!({
            "$id": bs58::encode([1u8; 32]).into_string(),
            "$ownerId": bs58::encode([2u8; 32]).into_string(),
            "$creatorId": bs58::encode(creator.to_buffer()).into_string(),
        });

        let doc = DocumentV0::from_json_value::<String, _>(json_val, platform_version)
            .expect("from_json_value with creator_id should succeed");

        assert_eq!(doc.creator_id, Some(creator));
    }
}
