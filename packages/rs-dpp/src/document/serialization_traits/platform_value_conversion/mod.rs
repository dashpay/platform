mod v0;

pub use v0::*;

use crate::document::Document;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::document_type::random_document::CreateRandomDocument;
    use crate::document::{DocumentV0, DocumentV0Getters};
    use crate::serialization::ValueConvertible;
    use crate::tests::json_document::json_document_to_contract;
    use platform_value::Identifier;
    use platform_version::version::PlatformVersion;

    // After Phase D step 8 slice A, the Value-shape round-trip lives on
    // canonical `ValueConvertible` (`to_object` / `into_object` /
    // `from_object`). The `to_map_value` / `into_map_value` helpers on
    // this trait are tested below — they're the only methods that stay.

    // ================================================================
    //  Round-trip: Document -> Value -> Document via canonical traits
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

            let value = document.clone().into_object().expect("into_object");
            let recovered = Document::from_object(value).expect("from_object");

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

        let id_val = map.get("$id").expect("should have $id");
        match id_val {
            Value::Identifier(bytes) => {
                assert_eq!(Identifier::new(*bytes), original_id);
            }
            _ => panic!("$id should be an Identifier value"),
        }
    }

    // ================================================================
    //  from_object via canonical traits with minimal document
    // ================================================================

    #[test]
    fn from_object_with_minimal_data() {
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

        let document: Document = doc_v0.into();
        let value = document.clone().into_object().expect("into_object");
        let recovered = Document::from_object(value).expect("from_object");

        assert_eq!(recovered.id(), id);
        assert_eq!(recovered.owner_id(), owner_id);
    }
}
