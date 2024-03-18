mod v0;

use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::DocumentName;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

impl DocumentType {
    /// Retrieve document types from a value array.
    ///
    /// This method takes a data contract ID, an array of contract document types, definition references,
    /// documents keep history contract default, documents mutable contract default,
    /// and a platform version as input parameters and returns a map of document types
    /// extracted from the provided value array.
    ///
    /// The process of retrieving document types is versioned, and the version is determined
    /// by the platform version parameter. If the version is not supported, an error is returned.
    ///
    /// # Parameters
    ///
    /// * `data_contract_id`: Identifier for the data contract.
    /// * `contract_document_types_raw`: Vector representing the raw contract document types.
    /// * `definition_references`: BTreeMap representing the definition references.
    /// * `documents_keep_history_contract_default`: A boolean flag that specifies the document's keep history contract default.
    /// * `documents_mutable_contract_default`: A boolean flag that specifies the document's mutable contract default.
    /// * `platform_version`: The platform version being used.
    ///
    /// # Returns
    ///
    /// * `Result<BTreeMap<String, DocumentType>, ProtocolError>`: On success, a map of document types.
    ///   On failure, a ProtocolError.
    pub fn create_document_types_from_document_schemas(
        data_contract_id: Identifier,
        document_schemas: BTreeMap<DocumentName, Value>,
        schema_defs: Option<&BTreeMap<String, Value>>,
        documents_keep_history_contract_default: bool,
        documents_mutable_contract_default: bool,
        validate: bool,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .class_method_versions
            .create_document_types_from_document_schemas
        {
            0 => DocumentTypeV0::create_document_types_from_document_schemas_v0(
                data_contract_id,
                document_schemas,
                schema_defs,
                documents_keep_history_contract_default,
                documents_mutable_contract_default,
                validate,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "create_document_types_from_document_schemas".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use platform_value::Identifier;
    use std::ops::Deref;

    #[test]
    pub fn should_not_allow_creating_document_types_with_empty_schema() {
        let result = crate::data_contract::document_type::DocumentType::create_document_types_from_document_schemas(
            Identifier::random(),
            Default::default(),
            None,
            false,
            false,
            false,
            crate::version::PlatformVersion::latest(),
        );

        match result {
            Err(crate::ProtocolError::ConsensusError(e)) => match e.deref() {
                ConsensusError::BasicError(err) => match err {
                    BasicError::DataContractEmptySchemaError(_) => {}
                    _ => panic!("Expected DataContractEmptySchemaError"),
                },
                _ => panic!("Expected basic consensus error"),
            },
            _ => panic!("Expected consensus error"),
        }
    }
}
