use crate::data_contract::document_type::DocumentType;
use crate::data_contract::DataContract;
use crate::document::document_transition::document_base_transition::JsonValue;
use crate::document::property_names::FEATURE_VERSION;
use crate::document::v0::DocumentV0;
use crate::version::FeatureVersion;
use crate::ProtocolError;
use derive_more::From;
use platform_value::{Identifier, Value};
use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};
use std::convert::TryInto;
use crate::document::DocumentV0Methods;
use crate::state_transition::documents_batch_transition::document_transition::document_base_transition::JsonValue;

/// The property names of a document
pub mod property_names {
    pub const FEATURE_VERSION: &str = "$version";
    pub const ID: &str = "$id";
    pub const DATA_CONTRACT_ID: &str = "$dataContractId";
    pub const REVISION: &str = "$revision";
    pub const OWNER_ID: &str = "$ownerId";
    pub const CREATED_AT: &str = "$createdAt";
    pub const UPDATED_AT: &str = "$updatedAt";
}

pub const IDENTIFIER_FIELDS: [&str; 3] = [
    property_names::ID,
    property_names::OWNER_ID,
    property_names::DATA_CONTRACT_ID,
];

#[derive(Clone, Debug, PartialEq, From)]
pub enum Document {
    V0(DocumentV0),
}
impl Document {
    /// Return a value given the path to its key for a document type.
    pub fn get_raw_for_document_type(
        &self,
        key_path: &str,
        document_type: &DocumentType,
        owner_id: Option<[u8; 32]>,
    ) -> Result<Option<Vec<u8>>, ProtocolError> {
        match self {
            Document::V0(v0) => v0.get_raw_for_document_type(key_path, document_type, owner_id),
        }
    }

    /// Return a value given the path to its key and the document type for a contract.
    pub fn get_raw_for_contract(
        &self,
        key: &str,
        document_type_name: &str,
        contract: &DataContract,
        owner_id: Option<[u8; 32]>,
    ) -> Result<Option<Vec<u8>>, ProtocolError> {
        match self {
            Document::V0(v0) => {
                v0.get_raw_for_contract(key, document_type_name, contract, owner_id)
            }
        }
    }

    /// The document is only unique within the contract and document type.
    /// Hence we must include contract and document type information to get uniqueness.
    pub fn hash(
        &self,
        contract: &DataContract,
        document_type: &DocumentType,
    ) -> Result<Vec<u8>, ProtocolError> {
        match self {
            Document::V0(v0) => v0.hash(contract, document_type),
        }
    }

    /// Increment the revision of the document.
    pub fn increment_revision(&mut self) -> Result<(), ProtocolError> {
        match self {
            Document::V0(v0) => v0.increment_revision(),
        }
    }

    /// Get the identifiers and binary paths for a document type within a data contract.
    pub fn get_identifiers_and_binary_paths<'a>(
        version: FeatureVersion,
        data_contract: &'a DataContract,
        document_type_name: &'a str,
    ) -> Result<(HashSet<&'a str>, HashSet<&'a str>), ProtocolError> {
        match version {
            0 => DocumentV0::get_identifiers_and_binary_paths(data_contract, document_type_name),
            version => Err(ProtocolError::UnknownVersionError(format!(
                "version {version} not known for document"
            ))),
        }
    }

    /// Convert the document to JSON with identifiers using bytes.
    pub fn to_json_with_identifiers_using_bytes(&self) -> Result<JsonValue, ProtocolError> {
        match self {
            Document::V0(v0) => v0.to_json_with_identifiers_using_bytes(),
        }
    }

    /// Convert the document to a map value.
    pub fn to_map_value(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        match self {
            Document::V0(v0) => v0.to_map_value(),
        }
    }

    /// Convert the document to a map value consuming the document.
    pub fn into_map_value(self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        match self {
            Document::V0(v0) => v0.into_map_value(),
        }
    }

    /// Convert the document to a value consuming the document.
    pub fn into_value(self) -> Result<Value, ProtocolError> {
        match self {
            Document::V0(v0) => v0.into_value(),
        }
    }

    /// Convert the document to an object.
    pub fn to_object(&self) -> Result<Value, ProtocolError> {
        match self {
            Document::V0(v0) => v0.to_object(),
        }
    }

    /// Convert the document to a JSON value.
    pub fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        match self {
            Document::V0(v0) => v0.to_json(),
        }
    }
    /// Create a document from a JSON value.
    pub fn from_json_value<S>(mut document_value: JsonValue) -> Result<Self, ProtocolError>
    where
        for<'de> S: Deserialize<'de> + TryInto<Identifier, Error = ProtocolError>,
    {
        Ok(Document::V0(DocumentV0::from_json_value::<S>(
            document_value,
        )?))
    }

    /// Create a document from a platform value.
    pub fn from_platform_value(document_value: Value) -> Result<Self, ProtocolError> {
        let version: FeatureVersion = document_value.get_integer(FEATURE_VERSION)?;
        match version {
            0 => Ok(Document::V0(DocumentV0::from_platform_value(
                document_value,
            )?)),
            version => Err(ProtocolError::UnknownVersionError(format!(
                "version {version} not known for document for call from_platform_value"
            ))),
        }
    }

    /// Convert the document to a JSON value.
    pub fn serialize(&self) -> Result<JsonValue, ProtocolError> {
        match self {
            Document::V0(v0) => v0.to_json(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::document_type::random_document::CreateRandomDocument;
    use crate::data_contract::extra::common::json_document_to_contract;
    use regex::Regex;

    #[test]
    fn test_serialization() {
        let contract = json_document_to_contract(
            "../rs-dpp/src/tests/payloads/contract/dashpay-contract.json",
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

        let deserialized_document = document_type
            .document_from_bytes(serialized_document.as_slice())
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
