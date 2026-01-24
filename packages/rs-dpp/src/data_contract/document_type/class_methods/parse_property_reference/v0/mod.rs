use crate::data_contract::document_type::reference::{
    DocumentPropertyReference, DocumentPropertyReferenceTarget,
};
use crate::data_contract::document_type::{property_names, DocumentPropertyType, DocumentType};
use crate::data_contract::errors::DataContractError;
use crate::ProtocolError;
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::Value;
use std::collections::BTreeMap;

impl DocumentType {
    pub(super) fn parse_property_reference_v0(
        data_contract_system_version: u16,
        inner_properties: &BTreeMap<String, &Value>,
        property_type: &DocumentPropertyType,
    ) -> Result<Option<DocumentPropertyReference>, ProtocolError> {
        if data_contract_system_version < 2 {
            // refers_to only available on data contracts after system version 2.
            // This is because prior to system version 2 we could have arbitrary fields.
            // If someone had an arbitrary field refers_to it would not have been validated
            // properly to exist and could lead things into an incoherent state.
            return Ok(None);
        }
        let Some(refers_to_value) = inner_properties.get(property_names::REFERS_TO) else {
            return Ok(None);
        };

        if !matches!(property_type, DocumentPropertyType::Identifier) {
            return Err(DataContractError::InvalidContractStructure(
                "refersTo is only allowed on identifier properties".to_string(),
            )
            .into());
        }

        let refers_to_map = refers_to_value.to_btree_ref_string_map()?;

        let target = match refers_to_map
            .get_str(property_names::TYPE)
            .map_err(|e| DataContractError::ValueWrongType(e.to_string()))?
        {
            "identity" => DocumentPropertyReferenceTarget::IdentityReferenceTarget,
            "contract" => DocumentPropertyReferenceTarget::ContractReferenceTarget,
            other => {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "invalid refersTo type {other}"
                ))
                .into());
            }
        };

        let must_exist = refers_to_map
            .get_optional_bool("mustExist")
            .map_err(|e| DataContractError::ValueWrongType(e.to_string()))?
            .unwrap_or(true);

        Ok(Some(DocumentPropertyReference { target, must_exist }))
    }
}
