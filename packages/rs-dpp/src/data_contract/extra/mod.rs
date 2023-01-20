use std::collections::BTreeMap;

use ciborium::value::Value as CborValue;

use crate::data_contract::document_type::mutability::{ContractConfig, DEFAULT_CONTRACT_CAN_BE_DELETED};
use crate::data_contract::document_type::mutability::{
    DEFAULT_CONTRACT_DOCUMENT_MUTABILITY, DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY,
    DEFAULT_CONTRACT_KEEPS_HISTORY, DEFAULT_CONTRACT_MUTABILITY,
};
use crate::data_contract::document_type::document_type::DocumentType;
use crate::data_contract::document_type::mutability;
use crate::data_contract::extra::common::cbor_map_to_btree_map;
use crate::ProtocolError;

pub mod common;

mod drive_api;
mod errors;

pub fn get_mutability(
    contract: &BTreeMap<String, CborValue>,
) -> Result<ContractConfig, ProtocolError> {
    let keeps_history: bool = common::bool_for_system_value_from_tree_map(
        contract,
        mutability::property::KEEPS_HISTORY,
        DEFAULT_CONTRACT_KEEPS_HISTORY,
    )?;
    let can_be_deleted: bool = common::bool_for_system_value_from_tree_map(
        contract,
        mutability::property::CAN_BE_DELETED,
        DEFAULT_CONTRACT_CAN_BE_DELETED,
    )?;
    let readonly: bool = common::bool_for_system_value_from_tree_map(
        contract,
        mutability::property::READONLY,
        !DEFAULT_CONTRACT_MUTABILITY,
    )?;
    let documents_keep_history_contract_default: bool =
        common::bool_for_system_value_from_tree_map(
            contract,
            mutability::property::DOCUMENTS_KEEP_HISTORY_CONTRACT_DEFAULT,
            DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY,
        )?;
    let documents_mutable_contract_default: bool = common::bool_for_system_value_from_tree_map(
        contract,
        mutability::property::DOCUMENTS_MUTABLE_CONTRACT_DEFAULT,
        DEFAULT_CONTRACT_DOCUMENT_MUTABILITY,
    )?;

    Ok(ContractConfig {
        can_be_deleted,
        readonly,
        keeps_history,
        documents_keep_history_contract_default,
        documents_mutable_contract_default,
    })
}

pub fn get_document_types(
    contract: &BTreeMap<String, CborValue>,
    definition_references: BTreeMap<String, &CborValue>,
    documents_keep_history_contract_default: bool,
    documents_mutable_contract_default: bool,
) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
    let documents_cbor_value = contract
        .get("documents")
        .ok_or(ContractError::MissingRequiredKey("unable to get documents"))?;
    let contract_document_types_raw =
        documents_cbor_value
            .as_map()
            .ok_or(ContractError::InvalidContractStructure(
                "documents must be a map",
            ))?;
    let mut contract_document_types: BTreeMap<String, DocumentType> = BTreeMap::new();
    for (type_key_value, document_type_value) in contract_document_types_raw {
        let Some(type_key_str) = type_key_value.as_text() else {
            return Err(ContractError::InvalidContractStructure(
                "document type name is not a string as expected",
            ));
        };

        // Make sure the document_type_value is a map
        let Some(document_type_raw_value_map) = document_type_value.as_map() else {
            return Err(ContractError::InvalidContractStructure(
                "document type data is not a map as expected",
            ));
        };

        let document_type_value_map = cbor_map_to_btree_map(document_type_raw_value_map);

        let document_type = DocumentType::from_cbor_value(
            type_key_str,
            document_type_value_map,
            &definition_references,
            documents_keep_history_contract_default,
            documents_mutable_contract_default,
        )?;
        contract_document_types.insert(
            type_key_str.to_string(),
            document_type,
        );
    }
    Ok(contract_document_types)
}

pub fn get_definitions(contract: &BTreeMap<String, CborValue>) -> BTreeMap<String, &CborValue> {
    let definition_references = match contract.get("$defs") {
        None => BTreeMap::new(),
        Some(definition_value) => {
            let definition_map = definition_value.as_map();
            match definition_map {
                None => BTreeMap::new(),
                Some(key_value) => common::cbor_map_to_btree_map(key_value),
            }
        }
    };
    definition_references
}
