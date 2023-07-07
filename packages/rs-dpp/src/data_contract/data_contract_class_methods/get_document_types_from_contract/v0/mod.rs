use crate::data_contract::document_type::DocumentType;
use crate::prelude::DataContract;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

impl DataContract {
    pub(super) fn get_document_types_from_contract_v0(
        data_contract_id: Identifier,
        contract: &BTreeMap<String, Value>,
        definition_references: &BTreeMap<String, &Value>,
        documents_keep_history_contract_default: bool,
        documents_mutable_contract_default: bool,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
        let Some(documents_value) =
            contract
                .get("documents") else {
            return Ok(BTreeMap::new());
        };
        DataContract::get_document_types_from_value(
            data_contract_id,
            documents_value,
            definition_references,
            documents_keep_history_contract_default,
            documents_mutable_contract_default,
            platform_version,
        )
    }
}
