use crate::consensus::basic::data_contract::DocumentTypesAreMissingError;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::class_methods::consensus_or_protocol_data_contract_error;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::{DocumentName, TokenConfiguration, TokenContractPosition};
use crate::validation::operations::ProtocolValidationOperation;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

impl DocumentType {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::data_contract) fn create_document_types_from_document_schemas_v1(
        data_contract_id: Identifier,
        data_contract_system_version: u16,
        contract_config_version: u16,
        document_schemas: BTreeMap<DocumentName, Value>,
        schema_defs: Option<&BTreeMap<String, Value>>,
        token_configurations: &BTreeMap<TokenContractPosition, TokenConfiguration>,
        data_contact_config: &DataContractConfig,
        full_validation: bool,
        has_tokens: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
        let mut contract_document_types: BTreeMap<String, DocumentType> = BTreeMap::new();

        if document_schemas.is_empty() && !has_tokens {
            return Err(consensus_or_protocol_data_contract_error(
                DocumentTypesAreMissingError::new(data_contract_id).into(),
            ));
        }

        for (name, schema) in document_schemas.into_iter() {
            let document_type = match platform_version
                .dpp
                .contract_versions
                .document_type_versions
                .structure_version
            {
                0 => DocumentType::try_from_schema(
                    data_contract_id,
                    data_contract_system_version,
                    contract_config_version,
                    &name,
                    schema,
                    schema_defs,
                    token_configurations,
                    data_contact_config,
                    full_validation,
                    validation_operations,
                    platform_version,
                )?,
                version => {
                    return Err(ProtocolError::UnknownVersionMismatch {
                        method: "get_document_types_from_value_array_v0 inner document type"
                            .to_string(),
                        known_versions: vec![0],
                        received: version,
                    })
                }
            };

            contract_document_types.insert(name.to_string(), document_type);
        }
        Ok(contract_document_types)
    }
}
