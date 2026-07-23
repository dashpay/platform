pub use fields::{property_names, IDENTIFIER_FIELDS};

mod accessors;
pub mod document_event;
#[cfg(feature = "client")]
mod document_facade;
#[cfg(feature = "factories")]
pub mod document_factory;
pub mod document_methods;
mod document_patch;
pub mod errors;
#[cfg(feature = "extended-document")]
pub mod extended_document;
mod fields;
pub mod generate_document_id;
pub mod serialization_traits;
#[cfg(feature = "factories")]
pub mod specialized_document_factory;
pub mod transfer;
mod v0;

pub use accessors::*;
pub use v0::*;

#[cfg(feature = "extended-document")]
pub use extended_document::property_names as extended_document_property_names;
#[cfg(feature = "extended-document")]
pub use extended_document::ExtendedDocument;
#[cfg(feature = "extended-document")]
pub use extended_document::IDENTIFIER_FIELDS as EXTENDED_DOCUMENT_IDENTIFIER_FIELDS;

/// the initial revision of newly created document
pub const INITIAL_REVISION: u64 = 1;

use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::DataContract;
use crate::document::document_methods::{
    DocumentGetRawForContractV0, DocumentGetRawForDocumentTypeV0, DocumentHashV0Method,
    DocumentIsEqualIgnoringTimestampsV0, DocumentMethodsV0,
};
use crate::document::errors::DocumentError;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use derive_more::From;

use std::fmt;
use std::fmt::Formatter;

#[derive(Clone, Debug, PartialEq, From)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "$formatVersion")
)]
pub enum Document {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(DocumentV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for Document {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for Document {}

impl fmt::Display for Document {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Document::V0(v0) => {
                write!(f, "v0 : {} ", v0)?;
            }
        }
        Ok(())
    }
}

impl DocumentMethodsV0 for Document {
    /// Return a value given the path to its key and the document type for a contract.
    fn get_raw_for_contract(
        &self,
        key: &str,
        document_type_name: &str,
        contract: &DataContract,
        owner_id: Option<[u8; 32]>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<u8>>, ProtocolError> {
        match self {
            Document::V0(document_v0) => {
                match platform_version
                    .dpp
                    .document_versions
                    .document_method_versions
                    .get_raw_for_contract
                {
                    0 => document_v0.get_raw_for_contract_v0(
                        key,
                        document_type_name,
                        contract,
                        owner_id,
                        platform_version,
                    ),
                    version => Err(ProtocolError::UnknownVersionMismatch {
                        method: "DocumentMethodV0::get_raw_for_contract".to_string(),
                        known_versions: vec![0],
                        received: version,
                    }),
                }
            }
        }
    }

    /// Return a value given the path to its key for a document type.
    fn get_raw_for_document_type(
        &self,
        key_path: &str,
        document_type: DocumentTypeRef,
        owner_id: Option<[u8; 32]>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<u8>>, ProtocolError> {
        match self {
            Document::V0(document_v0) => {
                match platform_version
                    .dpp
                    .document_versions
                    .document_method_versions
                    .get_raw_for_document_type
                {
                    0 => document_v0.get_raw_for_document_type_v0(
                        key_path,
                        document_type,
                        owner_id,
                        platform_version,
                    ),
                    version => Err(ProtocolError::UnknownVersionMismatch {
                        method: "DocumentMethodV0::get_raw_for_document_type".to_string(),
                        known_versions: vec![0],
                        received: version,
                    }),
                }
            }
        }
    }

    fn hash(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match self {
            Document::V0(document_v0) => {
                match platform_version
                    .dpp
                    .document_versions
                    .document_method_versions
                    .hash
                {
                    0 => document_v0.hash_v0(contract, document_type, platform_version),
                    version => Err(ProtocolError::UnknownVersionMismatch {
                        method: "DocumentMethodV0::hash".to_string(),
                        known_versions: vec![0],
                        received: version,
                    }),
                }
            }
        }
    }

    fn increment_revision(&mut self) -> Result<(), ProtocolError> {
        let Some(revision) = self.revision() else {
            return Err(ProtocolError::Document(Box::new(
                DocumentError::DocumentNoRevisionError {
                    document: Box::new(self.clone()),
                },
            )));
        };

        let new_revision = revision
            .checked_add(1)
            .ok_or(ProtocolError::Overflow("overflow when adding 1"))?;

        self.set_revision(Some(new_revision));

        Ok(())
    }

    fn is_equal_ignoring_time_based_fields(
        &self,
        rhs: &Self,
        also_ignore_fields: Option<Vec<&str>>,
        platform_version: &PlatformVersion,
    ) -> Result<bool, ProtocolError> {
        match (self, rhs) {
            (Document::V0(document_v0), Document::V0(rhs_v0)) => {
                match platform_version
                    .dpp
                    .document_versions
                    .document_method_versions
                    .is_equal_ignoring_timestamps
                {
                    0 => Ok(document_v0
                        .is_equal_ignoring_time_based_fields_v0(rhs_v0, also_ignore_fields)),
                    version => Err(ProtocolError::UnknownVersionMismatch {
                        method: "DocumentMethodV0::is_equal_ignoring_time_based_fields".to_string(),
                        known_versions: vec![0],
                        received: version,
                    }),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::document_type::random_document::CreateRandomDocument;
    use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use crate::tests::json_document::json_document_to_contract;

    use regex::Regex;

    #[test]
    fn test_document_display() {
        let platform_version = PlatformVersion::first();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected to get contract");

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get profile document type");
        let document = document_type
            .random_document(Some(3333), platform_version)
            .expect("expected to get a random document");

        let document_string = format!("{}", document);
        let pattern = r"v\d+ : id:45ZNwGcxeMpLpYmiVEKKBKXbZfinrhjZLkau1GWizPFX owner_id:2vq574DjKi7ZD8kJ6dMHxT5wu6ZKD2bW5xKAyKAGW7qZ created_at:(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) updated_at:(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) avatarUrl:string y8RD1DbW18RuyblDX7hx\[...\(670\)\] displayName:string y94Itl6mn1yBE publicMessage:string SvAQrzsslj0ESc15GQBQ\[...\(105\)\] .*";
        let re = Regex::new(pattern).unwrap();
        assert!(
            re.is_match(document_string.as_str()),
            "pattern: {} does not match {}",
            pattern,
            document_string
        );
    }

    #[test]
    fn test_serialization_and_deserialization() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dpns/dpns-contract.json",
            false,
            platform_version,
        )
        .expect("expected to get contract");

        let document_type = contract
            .document_type_for_name("domain")
            .expect("expected to get document type");
        for _ in 0..20 {
            let document = document_type
                .random_document(None, platform_version)
                .expect("expected a document");
            let serialized = <Document as DocumentPlatformConversionMethodsV0>::serialize(
                &document,
                document_type,
                &contract,
                platform_version,
            )
            .expect("should serialize");
            let _deserialized = Document::from_bytes(&serialized, document_type, platform_version)
                .expect("expected to deserialize domain document");
        }
    }

    #[test]
    fn test_serialize_deserialize_over_different_versions_of_document_type() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dpns/dpns-contract.json",
            false,
            platform_version,
        )
        .expect("expected to get contract");

        let updated_contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dpns/dpns-contract-update-v2-test.json",
            false,
            platform_version,
        )
        .expect("expected to get contract");

        let document_type = contract
            .document_type_for_name("domain")
            .expect("expected to get document type");

        let updated_document_type = updated_contract
            .document_type_for_name("domain")
            .expect("expected to get document type");

        // let's test from a document created in the old version, and we try to deserialize it in the new version
        for _ in 0..20 {
            let document = document_type
                .random_document(None, platform_version)
                .expect("expected a document");
            let serialized = <Document as DocumentPlatformConversionMethodsV0>::serialize(
                &document,
                document_type,
                &contract,
                platform_version,
            )
            .expect("should serialize");
            let _deserialized =
                Document::from_bytes(&serialized, updated_document_type, platform_version)
                    .expect("expected to deserialize domain document");
        }

        // let's test from a document created in the new version, and we try to deserialize it with the old version
        for _ in 0..20 {
            let document = updated_document_type
                .random_document(None, platform_version)
                .expect("expected a document");
            let serialized = <Document as DocumentPlatformConversionMethodsV0>::serialize(
                &document,
                document_type,
                &contract,
                platform_version,
            )
            .expect("should serialize");
            let _deserialized = Document::from_bytes(&serialized, document_type, platform_version)
                .expect("expected to deserialize domain document");
        }
    }

    // ================================================================
    //  Display impl tests for Document
    // ================================================================

    #[test]
    fn display_document_with_no_properties() {
        let doc = Document::V0(DocumentV0 {
            id: platform_value::Identifier::new([0xAA; 32]),
            owner_id: platform_value::Identifier::new([0xBB; 32]),
            properties: Default::default(),
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
        });

        let s = format!("{}", doc);
        assert!(
            s.contains("no properties"),
            "should say 'no properties' when the BTreeMap is empty, got: {}",
            s
        );
    }

    #[test]
    fn display_document_shows_transferred_at_fields() {
        let doc = Document::V0(DocumentV0 {
            id: platform_value::Identifier::new([1u8; 32]),
            owner_id: platform_value::Identifier::new([2u8; 32]),
            properties: Default::default(),
            revision: None,
            created_at: None,
            updated_at: None,
            transferred_at: Some(1_700_000_000_000),
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: Some(500),
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: Some(42),
            creator_id: None,
        });

        let s = format!("{}", doc);
        assert!(
            s.contains("transferred_at:"),
            "should contain transferred_at, got: {}",
            s
        );
        assert!(
            s.contains("transferred_at_block_height:500"),
            "should contain transferred_at_block_height:500, got: {}",
            s
        );
        assert!(
            s.contains("transferred_at_core_block_height:42"),
            "should contain transferred_at_core_block_height:42, got: {}",
            s
        );
    }

    #[test]
    fn display_document_shows_creator_id() {
        let creator = platform_value::Identifier::new([0xCC; 32]);
        let doc = Document::V0(DocumentV0 {
            id: platform_value::Identifier::new([1u8; 32]),
            owner_id: platform_value::Identifier::new([2u8; 32]),
            properties: Default::default(),
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
            creator_id: Some(creator),
        });

        let s = format!("{}", doc);
        assert!(
            s.contains("creator_id:"),
            "should contain creator_id, got: {}",
            s
        );
    }

    #[test]
    fn display_document_shows_block_height_fields() {
        let doc = Document::V0(DocumentV0 {
            id: platform_value::Identifier::new([1u8; 32]),
            owner_id: platform_value::Identifier::new([2u8; 32]),
            properties: Default::default(),
            revision: None,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: Some(100),
            updated_at_block_height: Some(200),
            transferred_at_block_height: None,
            created_at_core_block_height: Some(50),
            updated_at_core_block_height: Some(60),
            transferred_at_core_block_height: None,
            creator_id: None,
        });

        let s = format!("{}", doc);
        assert!(s.contains("created_at_block_height:100"), "got: {}", s);
        assert!(s.contains("updated_at_block_height:200"), "got: {}", s);
        assert!(s.contains("created_at_core_block_height:50"), "got: {}", s);
        assert!(s.contains("updated_at_core_block_height:60"), "got: {}", s);
    }

    // ================================================================
    //  Version dispatch: increment_revision
    // ================================================================

    #[test]
    fn increment_revision_works_on_mutable_document() {
        let mut doc = Document::V0(DocumentV0 {
            id: platform_value::Identifier::new([1u8; 32]),
            owner_id: platform_value::Identifier::new([2u8; 32]),
            properties: Default::default(),
            revision: Some(1),
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
        });

        doc.increment_revision()
            .expect("increment_revision should succeed");
        assert_eq!(doc.revision(), Some(2));
    }

    #[test]
    fn increment_revision_fails_when_no_revision() {
        let mut doc = Document::V0(DocumentV0 {
            id: platform_value::Identifier::new([1u8; 32]),
            owner_id: platform_value::Identifier::new([2u8; 32]),
            properties: Default::default(),
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
        });

        let result = doc.increment_revision();
        assert!(
            result.is_err(),
            "increment_revision should fail when revision is None"
        );
    }

    // ================================================================
    //  Version dispatch: is_equal_ignoring_time_based_fields
    // ================================================================

    #[test]
    fn is_equal_ignoring_time_based_fields_dispatches_correctly() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected to get contract");

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get profile document type");

        let doc1 = document_type
            .random_document(Some(42), platform_version)
            .expect("expected random document");

        let mut doc2 = doc1.clone();
        // Change timestamps
        doc2.set_created_at(Some(9_999_999));
        doc2.set_updated_at(Some(8_888_888));

        let result = doc1
            .is_equal_ignoring_time_based_fields(&doc2, None, platform_version)
            .expect("should succeed");
        assert!(
            result,
            "same document with different timestamps should be equal ignoring time fields"
        );
    }

    // ================================================================
    //  Version dispatch: get_raw_for_contract
    // ================================================================

    #[test]
    fn get_raw_for_contract_dispatches_to_v0() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected to get contract");

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get profile document type");

        let document = document_type
            .random_document(Some(7), platform_version)
            .expect("expected random document");

        let raw_id = document
            .get_raw_for_contract("$id", "profile", &contract, None, platform_version)
            .expect("should succeed");
        assert_eq!(raw_id, Some(document.id().to_vec()));
    }

    // ================================================================
    //  Version dispatch: hash
    // ================================================================

    #[test]
    fn document_hash_is_deterministic() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected to get contract");

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get profile document type");

        let document = document_type
            .random_document(Some(42), platform_version)
            .expect("expected random document");

        let hash1 = document
            .hash(&contract, document_type, platform_version)
            .expect("hash should succeed");
        let hash2 = document
            .hash(&contract, document_type, platform_version)
            .expect("hash should succeed");
        assert_eq!(hash1, hash2, "hash should be deterministic");
        assert!(!hash1.is_empty(), "hash should not be empty");
    }

    // ================================================================
    //  increment_revision: overflow from Revision::MAX surfaces an
    //  Overflow ProtocolError (not a silent saturate — this is the
    //  Document-enum path which uses checked_add).
    // ================================================================

    #[test]
    fn increment_revision_errors_on_overflow() {
        let mut doc = Document::V0(DocumentV0 {
            id: platform_value::Identifier::new([1u8; 32]),
            owner_id: platform_value::Identifier::new([2u8; 32]),
            properties: Default::default(),
            revision: Some(crate::prelude::Revision::MAX),
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
        });
        let err = doc.increment_revision().expect_err("MAX + 1 must overflow");
        match err {
            ProtocolError::Overflow(_) => {}
            other => panic!("expected ProtocolError::Overflow, got {:?}", other),
        }
    }

    // ================================================================
    //  From<DocumentV0> for Document produces a V0 variant.
    // ================================================================

    #[test]
    fn from_document_v0_produces_v0_variant() {
        let v0 = DocumentV0 {
            id: platform_value::Identifier::new([1u8; 32]),
            owner_id: platform_value::Identifier::new([2u8; 32]),
            properties: Default::default(),
            revision: Some(7),
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
        let document: Document = v0.clone().into();
        match document {
            Document::V0(inner) => assert_eq!(inner, v0),
        }
    }

    // ================================================================
    //  Document Display forwards to DocumentV0 Display with a version
    //  prefix.
    // ================================================================

    #[test]
    fn document_display_has_version_prefix() {
        let doc = Document::V0(DocumentV0 {
            id: platform_value::Identifier::new([1u8; 32]),
            owner_id: platform_value::Identifier::new([2u8; 32]),
            properties: Default::default(),
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
        });
        let s = format!("{}", doc);
        assert!(
            s.starts_with("v0 : "),
            "Display should prefix with version, got: {s}"
        );
    }

    // ================================================================
    //  get_raw_for_document_type dispatches via platform version 0
    //  to the V0 implementation.
    // ================================================================

    #[test]
    fn get_raw_for_document_type_dispatch_path_returns_id() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected contract");
        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected document type");

        let document = document_type
            .random_document(Some(11), platform_version)
            .expect("expected random document");

        let raw = document
            .get_raw_for_document_type("$id", document_type, None, platform_version)
            .expect("should succeed");
        assert_eq!(raw, Some(document.id().to_vec()));
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;

    use platform_value::{platform_value, Identifier};
    use serde_json::json;
    use std::collections::BTreeMap;

    fn fixture() -> Document {
        Document::V0(DocumentV0 {
            id: Identifier::new([0xa1; 32]),
            owner_id: Identifier::new([0xb2; 32]),
            properties: BTreeMap::new(),
            revision: Some(2),
            created_at: Some(1_700_000_000_000),
            updated_at: Some(1_700_000_001_000),
            transferred_at: None,
            created_at_block_height: Some(100),
            updated_at_block_height: Some(101),
            transferred_at_block_height: None,
            created_at_core_block_height: Some(50),
            updated_at_core_block_height: Some(51),
            transferred_at_core_block_height: None,
            creator_id: Some(Identifier::new([0xc3; 32])),
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Sized-int fields whose JSON wire encoding loses size info:
        // `$revision`/`$createdAt`/`$updatedAt`/`$createdAtBlockHeight`/
        // `$updatedAtBlockHeight` (u64), `$createdAtCoreBlockHeight`/
        // `$updatedAtCoreBlockHeight` (u32). The value-path locks variants
        // via explicit suffixes. `properties` is flattened into the document
        // root; for an empty `BTreeMap`, no extra keys appear.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "$id": Identifier::new([0xa1; 32]),
                "$ownerId": Identifier::new([0xb2; 32]),
                "$revision": 2,
                "$createdAt": 1_700_000_000_000u64,
                "$updatedAt": 1_700_000_001_000u64,
                "$transferredAt": serde_json::Value::Null,
                "$createdAtBlockHeight": 100,
                "$updatedAtBlockHeight": 101,
                "$transferredAtBlockHeight": serde_json::Value::Null,
                "$createdAtCoreBlockHeight": 50,
                "$updatedAtCoreBlockHeight": 51,
                "$transferredAtCoreBlockHeight": serde_json::Value::Null,
                "$creatorId": Identifier::new([0xc3; 32]),
            })
        );
        let recovered = Document::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // Explicit suffixes lock in sized variants: revision / *At /
        // *AtBlockHeight are u64; *AtCoreBlockHeight are u32.
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "$id": Identifier::new([0xa1; 32]),
                "$ownerId": Identifier::new([0xb2; 32]),
                "$revision": 2u64,
                "$createdAt": 1_700_000_000_000u64,
                "$updatedAt": 1_700_000_001_000u64,
                "$transferredAt": platform_value::Value::Null,
                "$createdAtBlockHeight": 100u64,
                "$updatedAtBlockHeight": 101u64,
                "$transferredAtBlockHeight": platform_value::Value::Null,
                "$createdAtCoreBlockHeight": 50u32,
                "$updatedAtCoreBlockHeight": 51u32,
                "$transferredAtCoreBlockHeight": platform_value::Value::Null,
                "$creatorId": Identifier::new([0xc3; 32]),
            })
        );
        let recovered = Document::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
