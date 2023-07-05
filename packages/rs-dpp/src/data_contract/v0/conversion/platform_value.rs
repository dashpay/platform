use std::collections::BTreeMap;
use platform_value::btreemap_extensions::{BTreeValueMapHelper, BTreeValueRemoveFromMapHelper};
use platform_value::Value;
use crate::data_contract::data_contract::DataContractV0;
use crate::data_contract::{get_contract_configuration_properties, get_definitions, get_document_types_from_contract, property_names};
use crate::data_contract::get_binary_properties_from_schema::get_binary_properties;
use crate::ProtocolError;

impl DataContractV0 {
    pub fn from_raw_object(raw_object: Value) -> Result<DataContractV0, ProtocolError> {
        let mut data_contract_map = raw_object
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;

        let id = data_contract_map
            .remove_identifier(property_names::ID)
            .map_err(ProtocolError::ValueError)?;

        let mutability = get_contract_configuration_properties(&data_contract_map)?;
        let definition_references = get_definitions(&data_contract_map)?;
        let document_types = get_document_types_from_contract(
            id,
            &data_contract_map,
            &definition_references,
            mutability.documents_keep_history_contract_default,
            mutability.documents_mutable_contract_default,
        )?;

        let documents = data_contract_map
            .remove(property_names::DOCUMENTS)
            .map(|value| value.try_into_validating_btree_map_json())
            .transpose()?
            .unwrap_or_default();

        let mutability = get_contract_configuration_properties(&data_contract_map)?;

        // Defs
        let defs =
            data_contract_map.get_optional_inner_str_json_value_map::<BTreeMap<_, _>>("$defs")?;

        let binary_properties = documents
            .iter()
            .map(|(doc_type, schema)| (String::from(doc_type), get_binary_properties(schema)))
            .collect();

        let data_contract = DataContractV0 {
            id,
            schema: data_contract_map
                .remove_string(property_names::SCHEMA)
                .map_err(ProtocolError::ValueError)?,
            version: data_contract_map
                .remove_integer(property_names::VERSION)
                .map_err(ProtocolError::ValueError)?,
            owner_id: data_contract_map
                .remove_identifier(property_names::OWNER_ID)
                .map_err(ProtocolError::ValueError)?,
            document_types,
            metadata: None,
            config: mutability,
            documents,
            defs,
            binary_properties,
        };

        Ok(data_contract)
    }
}