use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::errors::DataContractError;
use crate::prelude::DataContract;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

impl DataContract {
    pub(super) fn get_document_types_from_value_array_v0(
        data_contract_id: Identifier,
        contract_document_types_raw: &Vec<(&str, &Value)>,
        definition_references: &BTreeMap<String, &Value>,
        documents_keep_history_contract_default: bool,
        documents_mutable_contract_default: bool,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
        let mut contract_document_types: BTreeMap<String, DocumentType> = BTreeMap::new();
        for (type_key_str, document_type_value) in contract_document_types_raw {
            // Make sure the document_type_value is a map
            let Some(document_type_value_map) = document_type_value.as_map() else {
                return Err(ProtocolError::DataContractError(DataContractError::InvalidContractStructure(
                    "document type data is not a map as expected",
                )));
            };

            let document_type = match platform_version
                .dpp
                .contract_versions
                .document_type_versions
                .document_type_structure_version
            {
                0 => DocumentType::V0(DocumentTypeV0::from_platform_value(
                    data_contract_id,
                    type_key_str,
                    document_type_value_map,
                    definition_references,
                    documents_keep_history_contract_default,
                    documents_mutable_contract_default,
                    platform_version,
                )?),
                version => {
                    return Err(ProtocolError::UnknownVersionMismatch {
                        method: "get_document_types_from_value_array_v0 inner document type"
                            .to_string(),
                        known_versions: vec![0],
                        received: version,
                    })
                }
            };

            contract_document_types.insert(type_key_str.to_string(), document_type);
        }
        Ok(contract_document_types)
    }
}
