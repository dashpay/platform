use crate::data_contract::document_type::DocumentType;
use crate::data_contract::errors::DataContractError;
use crate::prelude::DataContract;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

impl DataContract {
    pub(super) fn get_document_types_from_value_v0(
        data_contract_id: Identifier,
        documents_value: &Value,
        definition_references: &BTreeMap<String, &Value>,
        documents_keep_history_contract_default: bool,
        documents_mutable_contract_default: bool,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
        let contract_document_types_raw =
            documents_value
                .as_map()
                .ok_or(ProtocolError::DataContractError(
                    DataContractError::InvalidContractStructure("documents must be a map"),
                ))?.iter().map(|(key, value)| {
                let Some(type_key_str) = key.as_text() else {
                    return Err(ProtocolError::DataContractError(DataContractError::InvalidContractStructure(
                        "document type name is not a string as expected",
                    )));
                };
                Ok((type_key_str, value))
            }).collect::<Result<Vec<(&str, &Value)>, ProtocolError>>()?;
        DataContract::create_document_types_from_document_schemas(
            data_contract_id,
            &contract_document_types_raw,
            definition_references,
            documents_keep_history_contract_default,
            documents_mutable_contract_default,
            platform_version,
        )
    }
}
