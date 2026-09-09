use crate::document::property_names;

use crate::identity::TimestampMillis;
use crate::prelude::{BlockHeight, CoreBlockHeight, Revision};

use crate::ProtocolError;

use crate::document::serialization_traits::DocumentCborMethodsV0;
use crate::document::v0::DocumentV0;
use crate::version::PlatformVersion;
use ciborium::Value as CborValue;
use integer_encoding::VarIntWriter;
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{Identifier, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};

#[cfg(feature = "cbor")]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DocumentForCbor {
    /// The unique document ID.
    #[serde(rename = "$id")]
    pub id: [u8; 32],

    /// The document's properties (data).
    #[serde(flatten)]
    pub properties: BTreeMap<String, CborValue>,

    /// The ID of the document's owner.
    #[serde(rename = "$ownerId")]
    pub owner_id: [u8; 32],

    /// The document revision.
    #[serde(rename = "$revision")]
    pub revision: Option<Revision>,

    #[serde(rename = "$createdAt")]
    pub created_at: Option<TimestampMillis>,
    #[serde(rename = "$updatedAt")]
    pub updated_at: Option<TimestampMillis>,
    #[serde(rename = "$transferredAt")]
    pub transferred_at: Option<TimestampMillis>,

    #[serde(rename = "$createdAtBlockHeight")]
    pub created_at_block_height: Option<BlockHeight>,
    #[serde(rename = "$updatedAtBlockHeight")]
    pub updated_at_block_height: Option<BlockHeight>,
    #[serde(rename = "$transferredAtBlockHeight")]
    pub transferred_at_block_height: Option<BlockHeight>,

    #[serde(rename = "$createdAtCoreBlockHeight")]
    pub created_at_core_block_height: Option<CoreBlockHeight>,
    #[serde(rename = "$updatedAtCoreBlockHeight")]
    pub updated_at_core_block_height: Option<CoreBlockHeight>,
    #[serde(rename = "$transferredAtCoreBlockHeight")]
    pub transferred_at_core_block_height: Option<CoreBlockHeight>,

    #[serde(rename = "$creatorId")]
    pub creator_id: Option<Identifier>,

    /// The contract-version stamp. Skipped when absent so pre-stamp CBOR
    /// output stays byte-identical; `default` keeps old CBOR readable.
    #[serde(
        rename = "$contractVersion",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub contract_version: Option<u32>,
}

#[cfg(feature = "cbor")]
impl TryFrom<DocumentV0> for DocumentForCbor {
    type Error = ProtocolError;

    fn try_from(value: DocumentV0) -> Result<Self, Self::Error> {
        let DocumentV0 {
            id,
            properties,
            owner_id,
            revision,
            created_at,
            updated_at,
            transferred_at,
            created_at_block_height,
            updated_at_block_height,
            transferred_at_block_height,
            created_at_core_block_height,
            updated_at_core_block_height,
            transferred_at_core_block_height,
            creator_id,
            contract_version,
        } = value;
        Ok(DocumentForCbor {
            contract_version,
            id: id.to_buffer(),
            properties: Value::convert_to_cbor_map(properties)
                .map_err(ProtocolError::ValueError)?,
            owner_id: owner_id.to_buffer(),
            revision,
            created_at,
            updated_at,
            transferred_at,
            created_at_block_height,
            updated_at_block_height,
            transferred_at_block_height,
            created_at_core_block_height,
            updated_at_core_block_height,
            transferred_at_core_block_height,
            creator_id,
        })
    }
}

impl DocumentV0 {
    /// Reads a CBOR-serialized document and creates a Document from it.
    /// If Document and Owner IDs are provided, they are used, otherwise they are created.
    fn from_map(
        mut document_map: BTreeMap<String, Value>,
        document_id: Option<[u8; 32]>,
        owner_id: Option<[u8; 32]>,
    ) -> Result<Self, ProtocolError> {
        let owner_id = match owner_id {
            None => document_map
                .remove_hash256_bytes(property_names::OWNER_ID)
                .map_err(ProtocolError::ValueError)?,
            Some(owner_id) => owner_id,
        };

        let id = match document_id {
            None => document_map
                .remove_hash256_bytes(property_names::ID)
                .map_err(ProtocolError::ValueError)?,
            Some(document_id) => document_id,
        };

        let revision = document_map.remove_optional_integer(property_names::REVISION)?;

        let created_at = document_map.remove_optional_integer(property_names::CREATED_AT)?;
        let updated_at = document_map.remove_optional_integer(property_names::UPDATED_AT)?;
        let transferred_at =
            document_map.remove_optional_integer(property_names::TRANSFERRED_AT)?;
        let created_at_block_height =
            document_map.remove_optional_integer(property_names::CREATED_AT_BLOCK_HEIGHT)?;
        let updated_at_block_height =
            document_map.remove_optional_integer(property_names::UPDATED_AT_BLOCK_HEIGHT)?;
        let transferred_at_block_height =
            document_map.remove_optional_integer(property_names::TRANSFERRED_AT_BLOCK_HEIGHT)?;
        let created_at_core_block_height =
            document_map.remove_optional_integer(property_names::CREATED_AT_CORE_BLOCK_HEIGHT)?;
        let updated_at_core_block_height =
            document_map.remove_optional_integer(property_names::UPDATED_AT_CORE_BLOCK_HEIGHT)?;
        let transferred_at_core_block_height = document_map
            .remove_optional_integer(property_names::TRANSFERRED_AT_CORE_BLOCK_HEIGHT)?;

        let creator_id = document_map
            .remove_optional_identifier(property_names::CREATOR_ID)
            .map_err(ProtocolError::ValueError)?;

        let contract_version =
            document_map.remove_optional_integer(property_names::CONTRACT_VERSION)?;

        // dev-note: properties is everything other than the id and owner id
        Ok(DocumentV0 {
            contract_version,
            properties: document_map,
            owner_id: Identifier::new(owner_id),
            id: Identifier::new(id),
            revision,
            created_at,
            updated_at,
            transferred_at,
            created_at_block_height,
            updated_at_block_height,
            transferred_at_block_height,
            created_at_core_block_height,
            updated_at_core_block_height,
            transferred_at_core_block_height,
            creator_id,
        })
    }
}

impl DocumentCborMethodsV0 for DocumentV0 {
    /// Reads a CBOR-serialized document and creates a Document from it.
    /// If Document and Owner IDs are provided, they are used, otherwise they are created.
    fn from_cbor(
        document_cbor: &[u8],
        document_id: Option<[u8; 32]>,
        owner_id: Option<[u8; 32]>,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        // first we need to deserialize the document and contract indices
        // we would need dedicated deserialization functions based on the document type
        let document_cbor_map: BTreeMap<String, CborValue> =
            ciborium::de::from_reader(document_cbor).map_err(|_| {
                ProtocolError::InvalidCBOR(
                    "unable to decode document for document call".to_string(),
                )
            })?;
        let document_map: BTreeMap<String, Value> =
            Value::convert_from_cbor_map(document_cbor_map).map_err(ProtocolError::ValueError)?;
        Self::from_map(document_map, document_id, owner_id)
    }

    fn to_cbor_value(&self) -> Result<CborValue, ProtocolError> {
        // After Phase D step 8 slice A, the V0 trait `to_object` was
        // deleted (1:1 canonical equivalent). Inline the body —
        // `IdentityPublicKeyV0` doesn't derive `ValueConvertible` directly
        // (only the outer `Document` enum does), so we go through
        // `platform_value::to_value` directly.
        let value = platform_value::to_value(self).map_err(ProtocolError::ValueError)?;
        value.try_into().map_err(ProtocolError::ValueError)
    }

    /// Serializes the Document to CBOR.
    fn to_cbor(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut buffer: Vec<u8> = Vec::new();
        buffer.write_varint(0).map_err(|_| {
            ProtocolError::EncodingError("error writing protocol version".to_string())
        })?;
        let cbor_document = DocumentForCbor::try_from(self.clone())?;
        ciborium::ser::into_writer(&cbor_document, &mut buffer).map_err(|_| {
            ProtocolError::EncodingError("unable to serialize into cbor".to_string())
        })?;
        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::document_type::random_document::CreateRandomDocument;
    use crate::document::serialization_traits::DocumentCborMethodsV0;
    use crate::document::DocumentV0Getters;
    use crate::tests::json_document::json_document_to_contract;
    use platform_version::version::PlatformVersion;

    fn make_document_v0_with_timestamps() -> DocumentV0 {
        let id = Identifier::new([1u8; 32]);
        let owner_id = Identifier::new([2u8; 32]);
        let mut properties = BTreeMap::new();
        properties.insert("name".to_string(), Value::Text("Alice".to_string()));
        properties.insert("age".to_string(), Value::U64(30));
        DocumentV0 {
            contract_version: None,
            id,
            owner_id,
            properties,
            revision: Some(1),
            created_at: Some(1_700_000_000_000),
            updated_at: Some(1_700_000_100_000),
            transferred_at: None,
            created_at_block_height: Some(100),
            updated_at_block_height: Some(200),
            transferred_at_block_height: None,
            created_at_core_block_height: Some(50),
            updated_at_core_block_height: Some(60),
            transferred_at_core_block_height: None,
            creator_id: None,
        }
    }

    // ================================================================
    //  Round-trip: to_cbor -> from_cbor preserves document data
    // ================================================================

    #[test]
    fn cbor_round_trip_preserves_contract_version_stamp() {
        use crate::document::Document;

        let platform_version = PlatformVersion::latest();
        let mut document = make_document_v0_with_timestamps();
        document.contract_version = Some(7);

        let cbor = document.to_cbor().expect("expected to serialize to cbor");
        let restored = Document::from_cbor(&cbor, None, None, platform_version)
            .expect("expected to deserialize from cbor");

        let Document::V0(restored) = restored;
        assert_eq!(restored.contract_version, Some(7));
        assert_eq!(restored.id, document.id);
        assert_eq!(restored.revision, document.revision);
        // (full property equality is not asserted: CBOR decodes integers as
        // I128, a pre-existing normalization of this legacy path)
        assert_eq!(
            restored.properties.get("name"),
            document.properties.get("name")
        );

        // An unstamped document round-trips to no stamp (the key is skipped
        // entirely when absent, keeping pre-stamp CBOR byte-identical)
        let unstamped = make_document_v0_with_timestamps();
        let unstamped_cbor = unstamped.to_cbor().expect("expected to serialize to cbor");
        let restored_unstamped = Document::from_cbor(&unstamped_cbor, None, None, platform_version)
            .expect("expected to deserialize from cbor");
        let Document::V0(restored_unstamped) = restored_unstamped;
        assert_eq!(restored_unstamped.contract_version, None);
    }

    #[test]
    fn cbor_round_trip_with_random_dashpay_profile() {
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

        for seed in 0..10u64 {
            let document = document_type
                .random_document(Some(seed), platform_version)
                .expect("expected random document");

            // Use Document-level from_cbor which handles the version prefix
            let cbor_bytes = document.to_cbor().expect("to_cbor should succeed");
            let recovered =
                crate::document::Document::from_cbor(&cbor_bytes, None, None, platform_version)
                    .expect("from_cbor should succeed");

            assert_eq!(document.id(), recovered.id(), "id mismatch for seed {seed}");
            assert_eq!(
                document.owner_id(),
                recovered.owner_id(),
                "owner_id mismatch for seed {seed}"
            );
            assert_eq!(
                document.revision(),
                recovered.revision(),
                "revision mismatch for seed {seed}"
            );
            assert_eq!(
                document.properties(),
                recovered.properties(),
                "properties mismatch for seed {seed}"
            );
        }
    }

    #[test]
    fn cbor_round_trip_with_explicit_ids_overrides_embedded_ids() {
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

        let document = document_type
            .random_document(Some(42), platform_version)
            .expect("expected random document");

        let cbor_bytes = document.to_cbor().expect("to_cbor should succeed");

        let override_id = [0xAA; 32];
        let override_owner = [0xBB; 32];

        let recovered = crate::document::Document::from_cbor(
            &cbor_bytes,
            Some(override_id),
            Some(override_owner),
            platform_version,
        )
        .expect("from_cbor with explicit ids should succeed");

        assert_eq!(
            recovered.id(),
            Identifier::new(override_id),
            "explicit document_id should override the one in CBOR"
        );
        assert_eq!(
            recovered.owner_id(),
            Identifier::new(override_owner),
            "explicit owner_id should override the one in CBOR"
        );
    }

    // ================================================================
    //  to_cbor_value produces a valid CborValue
    // ================================================================

    #[test]
    fn to_cbor_value_returns_map_for_document_with_properties() {
        let doc = make_document_v0_with_timestamps();
        let cbor_val = doc.to_cbor_value().expect("to_cbor_value should succeed");
        // CborValue should be a Map at the top level
        assert!(
            cbor_val.is_map(),
            "CBOR value of a document should be a Map, got {:?}",
            cbor_val
        );
    }

    // ================================================================
    //  to_cbor output starts with varint-encoded version prefix (0)
    // ================================================================

    #[test]
    fn to_cbor_starts_with_version_zero_varint() {
        let doc = make_document_v0_with_timestamps();
        let cbor_bytes = doc.to_cbor().expect("to_cbor should succeed");
        // The first byte should be the varint encoding of 0
        assert!(!cbor_bytes.is_empty(), "CBOR output should not be empty");
        assert_eq!(
            cbor_bytes[0], 0,
            "first byte should be varint(0) for version"
        );
    }

    // ================================================================
    //  from_cbor rejects invalid CBOR data
    // ================================================================

    #[test]
    fn from_cbor_rejects_invalid_cbor_bytes() {
        let platform_version = PlatformVersion::latest();
        let garbage = vec![0xFF, 0xFE, 0xFD, 0x00, 0x01];
        let result = DocumentV0::from_cbor(&garbage, None, None, platform_version);
        assert!(
            result.is_err(),
            "from_cbor should fail on invalid CBOR bytes"
        );
    }

    // ================================================================
    //  DocumentForCbor TryFrom preserves all timestamp fields
    // ================================================================

    #[test]
    fn document_for_cbor_preserves_all_fields() {
        let doc = make_document_v0_with_timestamps();
        let cbor_doc = DocumentForCbor::try_from(doc.clone()).expect("TryFrom should succeed");
        assert_eq!(cbor_doc.id, doc.id.to_buffer());
        assert_eq!(cbor_doc.owner_id, doc.owner_id.to_buffer());
        assert_eq!(cbor_doc.revision, doc.revision);
        assert_eq!(cbor_doc.created_at, doc.created_at);
        assert_eq!(cbor_doc.updated_at, doc.updated_at);
        assert_eq!(cbor_doc.transferred_at, doc.transferred_at);
        assert_eq!(
            cbor_doc.created_at_block_height,
            doc.created_at_block_height
        );
        assert_eq!(
            cbor_doc.updated_at_block_height,
            doc.updated_at_block_height
        );
        assert_eq!(
            cbor_doc.transferred_at_block_height,
            doc.transferred_at_block_height
        );
        assert_eq!(
            cbor_doc.created_at_core_block_height,
            doc.created_at_core_block_height
        );
        assert_eq!(
            cbor_doc.updated_at_core_block_height,
            doc.updated_at_core_block_height
        );
        assert_eq!(
            cbor_doc.transferred_at_core_block_height,
            doc.transferred_at_core_block_height
        );
    }

    // ================================================================
    //  from_map populates fields correctly from a BTreeMap<String, Value>
    // ================================================================

    #[test]
    fn from_map_extracts_system_fields_and_leaves_properties() {
        let id_bytes = [3u8; 32];
        let owner_bytes = [4u8; 32];

        let mut map = BTreeMap::new();
        map.insert(property_names::ID.to_string(), Value::Bytes32(id_bytes));
        map.insert(
            property_names::OWNER_ID.to_string(),
            Value::Bytes32(owner_bytes),
        );
        map.insert(property_names::REVISION.to_string(), Value::U64(5));
        map.insert(
            property_names::CREATED_AT.to_string(),
            Value::U64(1_000_000),
        );
        map.insert(
            property_names::UPDATED_AT.to_string(),
            Value::U64(2_000_000),
        );
        map.insert("customField".to_string(), Value::Text("hello".to_string()));

        let doc = DocumentV0::from_map(map, None, None).expect("from_map should succeed");

        assert_eq!(doc.id, Identifier::new(id_bytes));
        assert_eq!(doc.owner_id, Identifier::new(owner_bytes));
        assert_eq!(doc.revision, Some(5));
        assert_eq!(doc.created_at, Some(1_000_000));
        assert_eq!(doc.updated_at, Some(2_000_000));
        // The custom field should remain in properties
        assert_eq!(
            doc.properties.get("customField"),
            Some(&Value::Text("hello".to_string()))
        );
        // System fields should NOT be in properties
        assert!(!doc.properties.contains_key(property_names::ID));
        assert!(!doc.properties.contains_key(property_names::OWNER_ID));
        assert!(!doc.properties.contains_key(property_names::REVISION));
    }

    #[test]
    fn from_map_with_explicit_ids_overrides_map_ids() {
        let map_id = [10u8; 32];
        let map_owner = [11u8; 32];
        let override_id = [20u8; 32];
        let override_owner = [21u8; 32];

        let mut map = BTreeMap::new();
        map.insert(property_names::ID.to_string(), Value::Bytes32(map_id));
        map.insert(
            property_names::OWNER_ID.to_string(),
            Value::Bytes32(map_owner),
        );

        let doc = DocumentV0::from_map(map, Some(override_id), Some(override_owner))
            .expect("from_map should succeed");

        assert_eq!(
            doc.id,
            Identifier::new(override_id),
            "explicit document_id should take precedence"
        );
        assert_eq!(
            doc.owner_id,
            Identifier::new(override_owner),
            "explicit owner_id should take precedence"
        );
    }

    // ================================================================
    //  Round-trip via from_map: construct map, parse, verify
    // ================================================================

    // ================================================================
    //  from_map missing $id / $ownerId errors
    // ================================================================

    #[test]
    fn from_map_missing_id_fails_when_not_provided() {
        // Only owner_id in the map, no explicit document_id — from_map should
        // error on the ID extraction step.
        let mut map = BTreeMap::new();
        map.insert(
            property_names::OWNER_ID.to_string(),
            Value::Bytes32([4u8; 32]),
        );

        let result = DocumentV0::from_map(map, None, None);
        assert!(
            result.is_err(),
            "from_map without $id or explicit document_id should fail"
        );
    }

    #[test]
    fn from_map_missing_owner_id_fails_when_not_provided() {
        let mut map = BTreeMap::new();
        map.insert(property_names::ID.to_string(), Value::Bytes32([3u8; 32]));

        let result = DocumentV0::from_map(map, None, None);
        assert!(
            result.is_err(),
            "from_map without $ownerId or explicit owner_id should fail"
        );
    }

    // ================================================================
    //  from_map: creator id parsing
    // ================================================================

    #[test]
    fn from_map_extracts_creator_id_when_present_as_identifier() {
        let creator = Identifier::new([0xCD; 32]);
        let mut map = BTreeMap::new();
        map.insert(property_names::ID.to_string(), Value::Bytes32([1u8; 32]));
        map.insert(
            property_names::OWNER_ID.to_string(),
            Value::Bytes32([2u8; 32]),
        );
        map.insert(
            property_names::CREATOR_ID.to_string(),
            Value::Identifier(creator.to_buffer()),
        );

        let doc = DocumentV0::from_map(map, None, None).expect("from_map should succeed");
        assert_eq!(doc.creator_id, Some(creator));
    }

    #[test]
    fn from_map_creator_id_missing_stays_none() {
        let mut map = BTreeMap::new();
        map.insert(property_names::ID.to_string(), Value::Bytes32([1u8; 32]));
        map.insert(
            property_names::OWNER_ID.to_string(),
            Value::Bytes32([2u8; 32]),
        );

        let doc = DocumentV0::from_map(map, None, None).expect("from_map should succeed");
        assert_eq!(doc.creator_id, None);
    }

    // ================================================================
    //  from_cbor: bytes starting with truncated ciborium data fail
    // ================================================================

    #[test]
    fn from_cbor_rejects_empty_buffer() {
        let platform_version = PlatformVersion::latest();
        let result = DocumentV0::from_cbor(&[], None, None, platform_version);
        assert!(
            result.is_err(),
            "from_cbor should fail on an empty input buffer"
        );
    }

    #[test]
    fn from_cbor_rejects_truncated_map_bytes() {
        // A map header byte that claims to be a map but has no content.
        let platform_version = PlatformVersion::latest();
        let result = DocumentV0::from_cbor(&[0xA1], None, None, platform_version);
        assert!(
            result.is_err(),
            "from_cbor should fail on a truncated map prefix"
        );
    }

    // ================================================================
    //  to_cbor round-trip preserves owner_id/creator_id etc
    // ================================================================

    #[test]
    fn cbor_round_trip_via_to_cbor_and_from_cbor_preserves_fields() {
        let platform_version = PlatformVersion::latest();
        let doc = make_document_v0_with_timestamps();

        let bytes = doc.to_cbor().expect("to_cbor succeeds");
        let recovered = crate::document::Document::from_cbor(&bytes, None, None, platform_version)
            .expect("from_cbor succeeds");
        assert_eq!(doc.id, recovered.id());
        assert_eq!(doc.owner_id, recovered.owner_id());
        assert_eq!(doc.revision, recovered.revision());
    }

    // ================================================================
    //  DocumentForCbor: TryFrom returns a CBOR-ready structure whose
    //  `properties` map preserves user-defined keys.
    // ================================================================

    #[test]
    fn document_for_cbor_preserves_user_properties() {
        let doc = make_document_v0_with_timestamps();
        let cbor = DocumentForCbor::try_from(doc.clone()).expect("try_from succeeds");
        // "name" and "age" are part of the test fixture
        assert!(cbor.properties.contains_key("name"));
        assert!(cbor.properties.contains_key("age"));
    }

    #[test]
    fn from_map_with_all_timestamp_variants() {
        let mut map = BTreeMap::new();
        map.insert(property_names::ID.to_string(), Value::Bytes32([5u8; 32]));
        map.insert(
            property_names::OWNER_ID.to_string(),
            Value::Bytes32([6u8; 32]),
        );
        map.insert(
            property_names::CREATED_AT_BLOCK_HEIGHT.to_string(),
            Value::U64(100),
        );
        map.insert(
            property_names::UPDATED_AT_BLOCK_HEIGHT.to_string(),
            Value::U64(200),
        );
        map.insert(
            property_names::TRANSFERRED_AT.to_string(),
            Value::U64(3_000_000),
        );
        map.insert(
            property_names::TRANSFERRED_AT_BLOCK_HEIGHT.to_string(),
            Value::U64(300),
        );
        map.insert(
            property_names::CREATED_AT_CORE_BLOCK_HEIGHT.to_string(),
            Value::U32(50),
        );
        map.insert(
            property_names::UPDATED_AT_CORE_BLOCK_HEIGHT.to_string(),
            Value::U32(60),
        );
        map.insert(
            property_names::TRANSFERRED_AT_CORE_BLOCK_HEIGHT.to_string(),
            Value::U32(70),
        );

        let doc = DocumentV0::from_map(map, None, None).expect("from_map should succeed");

        assert_eq!(doc.created_at_block_height, Some(100));
        assert_eq!(doc.updated_at_block_height, Some(200));
        assert_eq!(doc.transferred_at, Some(3_000_000));
        assert_eq!(doc.transferred_at_block_height, Some(300));
        assert_eq!(doc.created_at_core_block_height, Some(50));
        assert_eq!(doc.updated_at_core_block_height, Some(60));
        assert_eq!(doc.transferred_at_core_block_height, Some(70));
    }
}
