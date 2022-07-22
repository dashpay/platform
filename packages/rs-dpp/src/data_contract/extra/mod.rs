pub mod common;

mod array_field;
mod document_field;
mod document_type;
mod drive_api;
mod errors;
mod index;
mod mutability;
mod root_tree;

use mutability::{
    DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY, DEFAULT_CONTRACT_DOCUMENT_MUTABILITY,
    DEFAULT_CONTRACT_KEEPS_HISTORY, DEFAULT_CONTRACT_MUTABILITY,
};
pub use {
    array_field::ArrayFieldType,
    document_field::{
        encode_float, encode_signed_integer, encode_unsigned_integer, DocumentField,
        DocumentFieldType,
    },
    document_type::DocumentType,
    drive_api::{DriveContractExt, DriveEncoding},
    errors::{ContractError, StructureError},
    index::{Index, IndexProperty},
    mutability::Mutability,
    root_tree::RootTree,
};

use ciborium::value::Value as CborValue;
use std::collections::BTreeMap;

pub fn get_mutability(contract: &BTreeMap<String, CborValue>) -> Result<Mutability, ContractError> {
    let keeps_history: bool = common::bool_for_system_value_from_tree_map(
        contract,
        "keepsHistory",
        DEFAULT_CONTRACT_KEEPS_HISTORY,
    )?;
    let readonly: bool = common::bool_for_system_value_from_tree_map(
        contract,
        "readOnly",
        !DEFAULT_CONTRACT_MUTABILITY,
    )?;
    let documents_keep_history_contract_default: bool =
        common::bool_for_system_value_from_tree_map(
            contract,
            "documentsKeepHistoryContractDefault",
            DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY,
        )?;
    let documents_mutable_contract_default: bool = common::bool_for_system_value_from_tree_map(
        contract,
        "documentsMutableContractDefault",
        DEFAULT_CONTRACT_DOCUMENT_MUTABILITY,
    )?;

    Ok(Mutability {
        keeps_history,
        readonly,
        documents_keep_history_contract_default,
        documents_mutable_contract_default,
    })
}

pub fn get_document_types(
    contract: &BTreeMap<String, CborValue>,
    definition_references: BTreeMap<String, &CborValue>,
    documents_keep_history_contract_default: bool,
    documents_mutable_contract_default: bool,
) -> Result<BTreeMap<String, DocumentType>, ContractError> {
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
        if !type_key_value.is_text() {
            return Err(ContractError::InvalidContractStructure(
                "document type name is not a string as expected",
            ));
        }

        // Make sure the document_type_value is a map
        if !document_type_value.is_map() {
            return Err(ContractError::InvalidContractStructure(
                "document type data is not a map as expected",
            ));
        }

        let document_type = DocumentType::from_cbor_value(
            type_key_value.as_text().expect("confirmed as text"),
            document_type_value.as_map().expect("confirmed as map"),
            &definition_references,
            documents_keep_history_contract_default,
            documents_mutable_contract_default,
        )?;
        contract_document_types.insert(
            String::from(type_key_value.as_text().expect("confirmed as text")),
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
