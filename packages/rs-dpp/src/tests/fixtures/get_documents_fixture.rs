use rand::rngs::StdRng;
use rand::SeedableRng;

use std::sync::Arc;

use platform_value::Value;
use serde_json::{json, Value as JsonValue};

use crate::contracts::withdrawals_contract::document_types;
use crate::data_contract::DriveContractExt;
use crate::document::Document;
use crate::{
    document::{
        document_factory::DocumentFactory,
        fetch_and_validate_data_contract::DataContractFetcherAndValidator,
    },
    prelude::*,
    state_repository::{MockStateRepositoryLike, StateRepositoryLike},
    tests::utils::generate_random_identifier_struct as gen_owner_id,
    version::LATEST_VERSION,
};

use super::get_document_validator_fixture;

pub fn get_documents_fixture_with_owner_id_from_contract(
    data_contract: DataContract,
) -> Result<Vec<ExtendedDocument>, ProtocolError> {
    let data_contract_fetcher_and_validator =
        DataContractFetcherAndValidator::new(Arc::new(MockStateRepositoryLike::new()));
    let factory = DocumentFactory::new(
        LATEST_VERSION,
        get_document_validator_fixture(),
        data_contract_fetcher_and_validator,
        None,
    );
    let owner_id = data_contract.owner_id.clone();

    get_extended_documents(factory, data_contract, owner_id)
}

pub fn get_documents_fixture(data_contract: DataContract) -> Result<Vec<Document>, ProtocolError> {
    Ok(get_extended_documents_fixture(data_contract)?
        .into_iter()
        .map(|extended_document| extended_document.document)
        .collect())
}

pub fn get_extended_documents_fixture(
    data_contract: DataContract,
) -> Result<Vec<ExtendedDocument>, ProtocolError> {
    let data_contract_fetcher_and_validator =
        DataContractFetcherAndValidator::new(Arc::new(MockStateRepositoryLike::new()));
    let factory = DocumentFactory::new(
        LATEST_VERSION,
        get_document_validator_fixture(),
        data_contract_fetcher_and_validator,
        None,
    );
    let owner_id = gen_owner_id();

    get_extended_documents(factory, data_contract, owner_id)
}

fn get_extended_documents<ST: StateRepositoryLike>(
    factory: DocumentFactory<ST>,
    data_contract: DataContract,
    owner_id: Identifier,
) -> Result<Vec<ExtendedDocument>, ProtocolError> {
    let documents = vec![
        factory.create_document_for_state_transition(
            data_contract.clone(),
            owner_id,
            "niceDocument".to_string(),
            json!({ "name": "Cutie" }).into(),
        )?,
        factory.create_document_for_state_transition(
            data_contract.clone(),
            owner_id,
            "prettyDocument".to_string(),
            json!({ "lastName": "Shiny" }).into(),
        )?,
        factory.create_document_for_state_transition(
            data_contract.clone(),
            owner_id,
            "prettyDocument".to_string(),
            json!({ "lastName": "Sweety" }).into(),
        )?,
        factory.create_document_for_state_transition(
            data_contract.clone(),
            owner_id,
            "indexedDocument".to_string(),
            json!( { "firstName": "William", "lastName": "Birkin" }).into(),
        )?,
        factory.create_document_for_state_transition(
            data_contract.clone(),
            owner_id,
            "indexedDocument".to_string(),
            json!( { "firstName": "Leon", "lastName": "Kennedy" }).into(),
        )?,
        factory.create_document_for_state_transition(
            data_contract.clone(),
            owner_id,
            "noTimeDocument".to_string(),
            json!({ "name": "ImOutOfTime" }).into(),
        )?,
        factory.create_document_for_state_transition(
            data_contract.clone(),
            owner_id,
            "uniqueDates".to_string(),
            json!({ "firstName": "John" }).into(),
        )?,
        factory.create_document_for_state_transition(
            data_contract.clone(),
            owner_id,
            "indexedDocument".to_string(),
            json!( { "firstName": "Bill", "lastName": "Gates" }).into(),
        )?,
        factory.create_document_for_state_transition(data_contract.clone(), owner_id, "withByteArrays".to_string(), json!( { "byteArrayField": get_random_10_bytes(), "identifierField": gen_owner_id().to_buffer() }).into())?,
        factory.create_document_for_state_transition(
            data_contract,
            owner_id,
            "optionalUniqueIndexedDocument".to_string(),
            json!({ "firstName": "Jacques-Yves", "lastName": "Cousteau" }).into()
        )?,
    ];

    Ok(documents)
}

pub fn get_withdrawal_document_fixture(
    data_contract: &DataContract,
    owner_id: Identifier,
    data: JsonValue,
    seed: Option<u64>,
) -> Result<Document, ProtocolError> {
    let mut rng = match seed {
        None => StdRng::from_entropy(),
        Some(seed_value) => StdRng::seed_from_u64(seed_value),
    };

    let document_type = data_contract.document_type_for_name(document_types::WITHDRAWAL)?;

    let value: Value = data.into();
    let properties = value.into_btree_map().map_err(ProtocolError::ValueError)?;

    let id = Identifier::random(&mut rng);
    document_type.create_document_with_valid_properties(id, owner_id, properties)
}

fn get_random_10_bytes() -> Vec<u8> {
    let mut buffer = [0u8; 10];
    let _ = getrandom::getrandom(&mut buffer);
    buffer.to_vec()
}
