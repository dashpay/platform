mod v0;
mod v1;

use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::{DocumentName, TokenConfiguration, TokenContractPosition};
use crate::validation::operations::ProtocolValidationOperation;
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
    #[allow(clippy::too_many_arguments)]
    pub fn create_document_types_from_document_schemas(
        data_contract_id: Identifier,
        document_schemas: BTreeMap<DocumentName, Value>,
        schema_defs: Option<&BTreeMap<String, Value>>,
        token_configurations: &BTreeMap<TokenContractPosition, TokenConfiguration>,
        data_contact_config: &DataContractConfig,
        full_validation: bool,
        has_tokens: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .class_method_versions
            .create_document_types_from_document_schemas
        {
            0 => DocumentType::create_document_types_from_document_schemas_v0(
                data_contract_id,
                document_schemas,
                schema_defs,
                token_configurations,
                data_contact_config,
                full_validation,
                validation_operations,
                platform_version,
            ),
            // in v1 we add the ability to have contracts without documents and just tokens
            1 => DocumentType::create_document_types_from_document_schemas_v1(
                data_contract_id,
                document_schemas,
                schema_defs,
                token_configurations,
                data_contact_config,
                full_validation,
                has_tokens,
                validation_operations,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "create_document_types_from_document_schemas".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}
