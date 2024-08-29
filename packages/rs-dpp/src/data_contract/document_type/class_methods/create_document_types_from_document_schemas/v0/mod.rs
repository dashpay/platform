use crate::consensus::basic::data_contract::DocumentTypesAreMissingError;
use crate::data_contract::document_type::class_methods::consensus_or_protocol_data_contract_error;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::DocumentName;
use crate::validation::operations::ProtocolValidationOperation;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

impl DocumentTypeV0 {
    pub(in crate::data_contract) fn create_document_types_from_document_schemas_v0(
        data_contract_id: Identifier,
        document_schemas: BTreeMap<DocumentName, Value>,
        schema_defs: Option<&BTreeMap<String, Value>>,
        documents_keep_history_contract_default: bool,
        documents_mutable_contract_default: bool,
        documents_can_be_deleted_contract_default: bool,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
        let mut contract_document_types: BTreeMap<String, DocumentType> = BTreeMap::new();

        if document_schemas.is_empty() {
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
                    &name,
                    schema,
                    schema_defs,
                    documents_keep_history_contract_default,
                    documents_mutable_contract_default,
                    documents_can_be_deleted_contract_default,
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::consensus::basic::data_contract::DocumentTypesAreMissingError;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::errors::DataContractError;
    use assert_matches::assert_matches;
    use platform_value::Identifier;
    use std::ops::Deref;

    #[test]
    pub fn should_not_allow_creating_document_types_with_empty_schema() {
        let id = Identifier::random();

        let result = DocumentType::create_document_types_from_document_schemas(
            id,
            Default::default(),
            None,
            false,
            false,
            false,
            false,
            &mut vec![],
            PlatformVersion::latest(),
        );

        assert_matches!(result, Err(ProtocolError::ConsensusError(e)) => {
            assert_matches!(e.deref(), ConsensusError::BasicError(BasicError::ContractError(
                DataContractError::DocumentTypesAreMissingError(
                    DocumentTypesAreMissingError { .. }
                )
            )));
        });
    }
}
