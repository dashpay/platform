use std::collections::BTreeMap;
use platform_value::{Identifier, Value};
use crate::data_contract::document_type::DocumentType;
use crate::prelude::DataContract;
use crate::ProtocolError;
use crate::version::PlatformVersion;

mod v0;

impl DataContract {
    /// Retrieve document types from a given value.
    ///
    /// This method takes a data contract identifier, a value representing document types,
    /// definition references, and several other parameters, and retrieves 
    /// the document types based on the values found in the map.
    ///
    /// The process of retrieving document types is versioned, 
    /// and the version is determined by the platform version parameter. 
    /// If the version is not supported, an error is returned.
    ///
    /// # Parameters
    ///
    /// * `data_contract_id`: The data contract identifier.
    /// * `documents_value`: The value representing document types.
    /// * `definition_references`: BTreeMap representing the definition references.
    /// * `documents_keep_history_contract_default`: A boolean indicating whether the documents keep history by contract default.
    /// * `documents_mutable_contract_default`: A boolean indicating whether the documents are mutable by contract default.
    /// * `platform_version`: The platform version being used.
    ///
    /// # Returns
    ///
    /// * `Result<BTreeMap<String, DocumentType>, ProtocolError>`: On success, a BTreeMap of document types.
    ///   On failure, a ProtocolError.
    pub fn get_document_types_from_value<'a>(
        data_contract_id: Identifier,
        documents_value: &'a Value,
        definition_references: &'a BTreeMap<String, &'a Value>,
        documents_keep_history_contract_default: bool,
        documents_mutable_contract_default: bool,
        platform_version: &'a PlatformVersion,
    ) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
        match platform_version.dpp.contract_versions.contract_class_method_versions.get_document_types_from_value {
            0 => Self::get_document_types_from_value_v0(
                data_contract_id,
                documents_value,
                definition_references,
                documents_keep_history_contract_default,
                documents_mutable_contract_default,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "get_document_types_from_value".to_string(),
                known_versions: vec![0],
                received: version,
            })
        }
    }
}