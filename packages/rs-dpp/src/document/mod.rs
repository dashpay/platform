pub use fields::{property_names, IDENTIFIER_FIELDS};

mod accessors;
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

#[cfg(feature = "document-serde-conversion")]
use serde::{Deserialize, Serialize};

use std::fmt;
use std::fmt::Formatter;

#[derive(Clone, Debug, PartialEq, From)]
#[cfg_attr(
    feature = "document-serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$version")
)]
pub enum Document {
    #[cfg_attr(feature = "document-serde-conversion", serde(rename = "0"))]
    V0(DocumentV0),
}

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

    fn is_equal_ignoring_timestamps(
        &self,
        rhs: &Self,
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
                    0 => Ok(document_v0.is_equal_ignoring_timestamps_v0(rhs_v0)),
                    version => Err(ProtocolError::UnknownVersionMismatch {
                        method: "DocumentMethodV0::is_equal_ignoring_timestamps".to_string(),
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
                platform_version,
            )
            .expect("should serialize");
            let _deserialized = Document::from_bytes(&serialized, document_type, platform_version)
                .expect("expected to deserialize domain document");
        }
    }
}
