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
#[cfg(feature = "json-object")]
mod json_conversion;
#[cfg(feature = "platform-value")]
mod platform_value_conversion;
mod serde_serialize;
pub mod serialization_traits;
pub mod serialize;
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
    DocumentMethodsV0,
};
use crate::document::errors::DocumentError;
use crate::version::{FeatureVersion, PlatformVersion};
use crate::ProtocolError;
use derive_more::From;
use platform_value::{Identifier, Value};
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug, PartialEq, From)]
pub enum Document {
    V0(DocumentV0),
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
    fn get_raw_for_document_type<'a>(
        &'a self,
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
            return Err(ProtocolError::Document(Box::new(DocumentError::DocumentNoRevisionError {
                document: Box::new(self.clone().into()),
            })))
        };

        let new_revision = revision
            .checked_add(1)
            .ok_or(ProtocolError::Overflow("overflow when adding 1"))?;

        self.set_revision(Some(new_revision));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::extra::common::json_document_to_contract;
    use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use regex::Regex;

    #[test]
    fn test_serialization() {
        let contract = json_document_to_contract(
            "../rs-dpp/src/tests/payloads/contract/dashpay-contract.json",
            0,
        )
        .expect("expected to get dashpay contract");

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected to get profile document type");
        let document = document_type.random_document(Some(3333));

        let document_cbor = document.to_cbor().expect("expected to encode to cbor");

        let serialized_document = document
            .serialize(document_type)
            .expect("expected to serialize");

        let document = Document::from_bytes(document_cbor, document_type, platform_version)
            .expect("expected to deserialize a document");
        assert_eq!(document, deserialized_document);
        assert!(serialized_document.len() < document_cbor.len());
        for _i in 0..10000 {
            let document = document_type.random_document(Some(3333));
            let _serialized_document = document
                .serialize_consume(document_type)
                .expect("expected to serialize");
        }
    }

    #[test]
    fn test_document_cbor_serialization() {
        let contract = json_document_to_contract(
            "../rs-dpp/src/tests/payloads/contract/dashpay-contract.json",
            0,
        )
        .expect("expected to get cbor contract");

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get profile document type");
        let document = document_type.random_document(Some(3333));

        let document_cbor = document.to_cbor().expect("expected to encode to cbor");

        let recovered_document = DocumentV0::from_cbor(document_cbor.as_slice(), None, None)
            .expect("expected to get document");

        assert_eq!(recovered_document, document);
    }

    #[test]
    fn test_document_display() {
        let contract = json_document_to_contract(
            "../rs-dpp/src/tests/payloads/contract/dashpay-contract.json",
            0,
        )
        .expect("expected to get contract");

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get profile document type");
        let document = document_type.random_document(Some(3333));

        let document_string = format!("{}", document);

        let pattern = r#"id:45ZNwGcxeMpLpYmiVEKKBKXbZfinrhjZLkau1GWizPFX owner_id:2vq574DjKi7ZD8kJ6dMHxT5wu6ZKD2bW5xKAyKAGW7qZ created_at:(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) updated_at:(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) avatarUrl:string y8RD1DbW18RuyblDX7hx\[...\(670\)\] displayName:string SvAQrzsslj0ESc15GQB publicMessage:string ccpKt9ckWftHIEKdBlas\[...\(36\)\] .*"#;
        let re = Regex::new(pattern).unwrap();
        assert!(re.is_match(document_string.as_str()));
    }
}
