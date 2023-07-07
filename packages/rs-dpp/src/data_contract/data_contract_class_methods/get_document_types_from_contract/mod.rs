mod v0;

use crate::data_contract::document_type::DocumentType;
use crate::prelude::DataContract;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

impl DataContract {
    /// Retrieve document types from a data contract.
    ///
    /// This method takes a data contract ID, a contract, definition references,
    /// documents keep history contract default, documents mutable contract default,
    /// and a platform version as input parameters and returns a map of document types
    /// extracted from the provided contract.
    ///
    /// The process of retrieving document types is versioned, and the version is determined
    /// by the platform version parameter. If the version is not supported, an error is returned.
    ///
    /// # Parameters
    ///
    /// * `data_contract_id`: Identifier for the data contract.
    /// * `contract`: BTreeMap representing the contract.
    /// * `definition_references`: BTreeMap representing the definition references.
    /// * `documents_keep_history_contract_default`: A boolean flag that specifies the document's keep history contract default.
    /// * `documents_mutable_contract_default`: A boolean flag that specifies the document's mutable contract default.
    /// * `platform_version`: The platform version being used.
    ///
    /// # Returns
    ///
    /// * `Result<BTreeMap<String, DocumentType>, ProtocolError>`: On success, a map of document types.
    ///   On failure, a ProtocolError.
    pub fn get_document_types_from_contract<'a>(
        data_contract_id: Identifier,
        contract: &'a BTreeMap<String, Value>,
        definition_references: &'a BTreeMap<String, &'a Value>,
        documents_keep_history_contract_default: bool,
        documents_mutable_contract_default: bool,
        platform_version: &'a PlatformVersion,
    ) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .contract_class_method_versions
            .get_document_types_from_contract
        {
            0 => Self::get_document_types_from_contract_v0(
                data_contract_id,
                contract,
                definition_references,
                documents_keep_history_contract_default,
                documents_mutable_contract_default,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "get_document_types_from_contract".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
