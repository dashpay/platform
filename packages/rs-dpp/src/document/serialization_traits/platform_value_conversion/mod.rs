mod v0;

pub use v0::*;

use crate::document::{Document, DocumentV0};
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;
use std::collections::BTreeMap;

impl DocumentPlatformValueMethodsV0<'_> for Document {
    /// Convert the document to a map value.
    fn to_map_value(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        match self {
            Document::V0(v0) => v0.to_map_value(),
        }
    }

    /// Convert the document to a map value consuming the document.
    fn into_map_value(self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        match self {
            Document::V0(v0) => v0.into_map_value(),
        }
    }

    /// Convert the document to a value consuming the document.
    fn into_value(self) -> Result<Value, ProtocolError> {
        match self {
            Document::V0(v0) => v0.into_value(),
        }
    }

    /// Convert the document to an object.
    fn to_object(&self) -> Result<Value, ProtocolError> {
        match self {
            Document::V0(v0) => v0.to_object(),
        }
    }

    /// Create a document from a platform value.
    fn from_platform_value(
        document_value: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .document_versions
            .document_structure_version
        {
            0 => Ok(Document::V0(DocumentV0::from_platform_value(
                document_value,
                platform_version,
            )?)),
            version => Err(ProtocolError::UnknownVersionError(format!(
                "version {version} not known for document for call from_platform_value"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::document_type::random_document::CreateRandomDocument;
    use crate::document::DocumentV0Getters;
    use crate::tests::json_document::json_document_to_contract;
    use platform_value::Identifier;
    use platform_version::version::PlatformVersion;

    // ================================================================
    //  Round-trip: Document -> Value -> Document
    // ================================================================

    #[test]
    fn round_trip_document_to_value_and_back() {
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

            let value = Document::into_value(document.clone()).expect("into_value should succeed");

            let recovered = Document::from_platform_value(value, platform_version)
                .expect("from_platform_value should succeed");

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

    // ================================================================
    //  to_map_value preserves all fields
    // ================================================================

    #[test]
    fn to_map_value_contains_id_and_owner_id() {
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

        let map = document
            .to_map_value()
            .expect("to_map_value should succeed");
        assert!(map.contains_key("$id"), "map should contain $id");
        assert!(map.contains_key("$ownerId"), "map should contain $ownerId");
    }

    // ================================================================
    //  to_object returns a Value
    // ================================================================

    #[test]
    fn to_object_returns_map_value() {
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
            .random_document(Some(7), platform_version)
            .expect("expected random document");

        let obj = document.to_object().expect("to_object should succeed");
        assert!(obj.is_map(), "to_object should return a Map value");
    }

    // ================================================================
    //  into_map_value consumes document
    // ================================================================

    #[test]
    fn into_map_value_consumes_and_returns_correct_data() {
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
            .random_document(Some(55), platform_version)
            .expect("expected random document");

        let original_id = document.id();
        let map = document
            .into_map_value()
            .expect("into_map_value should succeed");

        // The map should contain the id
        let id_val = map.get("$id").expect("should have $id");
        match id_val {
            Value::Identifier(bytes) => {
                assert_eq!(
                    Identifier::new(*bytes),
                    original_id,
                    "id in map should match original"
                );
            }
            _ => panic!("$id should be an Identifier value"),
        }
    }

    // ================================================================
    //  from_platform_value with minimal document
    // ================================================================

    #[test]
    fn from_platform_value_with_minimal_data() {
        let platform_version = PlatformVersion::latest();
        let id = Identifier::new([1u8; 32]);
        let owner_id = Identifier::new([2u8; 32]);

        let doc_v0 = DocumentV0 {
            id,
            owner_id,
            properties: std::collections::BTreeMap::new(),
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

        let value = DocumentV0::into_value(doc_v0).expect("into_value should succeed");
        let recovered = Document::from_platform_value(value, platform_version)
            .expect("from_platform_value should succeed");

        assert_eq!(recovered.id(), id);
        assert_eq!(recovered.owner_id(), owner_id);
    }
}
