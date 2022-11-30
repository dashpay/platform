use serde_json::{json, Value};

use crate::{
    contracts::withdrawals_contract, document::document_factory::DocumentFactory, mocks,
    prelude::*, tests::utils::generate_random_identifier_struct as gen_owner_id,
    version::LATEST_VERSION,
};

use super::get_document_validator_fixture;

pub fn get_documents_fixture_with_owner_id_from_contract(
    data_contract: DataContract,
) -> Result<Vec<Document>, ProtocolError> {
    let factory = DocumentFactory::new(
        LATEST_VERSION,
        get_document_validator_fixture(),
        mocks::FetchAndValidateDataContract {},
    );
    let owner_id = data_contract.owner_id().clone();

    get_documents(factory, data_contract, owner_id)
}

pub fn get_documents_fixture(data_contract: DataContract) -> Result<Vec<Document>, ProtocolError> {
    let factory = DocumentFactory::new(
        LATEST_VERSION,
        get_document_validator_fixture(),
        mocks::FetchAndValidateDataContract {},
    );
    let owner_id = gen_owner_id();

    get_documents(factory, data_contract, owner_id)
}

pub fn get_withdrawal_document_fixture(data_contract: &DataContract, data: Value) -> Document {
    let factory = DocumentFactory::new(
        LATEST_VERSION,
        get_document_validator_fixture(),
        mocks::FetchAndValidateDataContract {},
    );

    factory
        .create(
            data_contract.clone(),
            data_contract.owner_id.clone(),
            withdrawals_contract::types::WITHDRAWAL.to_string(),
            data,
        )
        .unwrap()
}

fn get_documents(
    factory: DocumentFactory,
    data_contract: DataContract,
    owner_id: Identifier,
) -> Result<Vec<Document>, ProtocolError> {
    let documents = vec![
        factory.create(
            data_contract.clone(),
            owner_id.clone(),
            "niceDocument".to_string(),
            json!({ "name": "Cutie" }),
        )?,
        factory.create(
            data_contract.clone(),
            owner_id.clone(),
            "prettyDocument".to_string(),
            json!({ "lastName": "Shiny" }),
        )?,
        factory.create(
            data_contract.clone(),
            owner_id.clone(),
            "prettyDocument".to_string(),
            json!({ "lastName": "Sweety" }),
        )?,
        factory.create(
            data_contract.clone(),
            owner_id.clone(),
            "indexedDocument".to_string(),
            json!( { "firstName": "William", "lastName": "Birkin" }),
        )?,
        factory.create(
            data_contract.clone(),
            owner_id.clone(),
            "indexedDocument".to_string(),
            json!( { "firstName": "Leon", "lastName": "Kennedy" }),
        )?,
        factory.create(
            data_contract.clone(),
            owner_id.clone(),
            "noTimeDocument".to_string(),
            json!({ "name": "ImOutOfTime" }),
        )?,
        factory.create(
            data_contract.clone(),
            owner_id.clone(),
            "uniqueDates".to_string(),
            json!({ "firstName": "John" }),
        )?,
        factory.create(
            data_contract.clone(),
            owner_id.clone(),
            "indexedDocument".to_string(),
            json!( { "firstName": "Bill", "lastName": "Gates" }),
        )?,
        factory.create(data_contract.clone(), owner_id.clone(), "withByteArrays".to_string(), json!( { "byteArrayField": get_random_10_bytes(), "identifierField": gen_owner_id().to_buffer() }),)?,
        factory.create(
            data_contract,
            owner_id,
            "optionalUniqueIndexedDocument".to_string(),
            json!({ "firstName": "Jacques-Yves", "lastName": "Cousteau" }),
        )?,
    ];

    Ok(documents)
}

fn get_random_10_bytes() -> Vec<u8> {
    let mut buffer = [0u8; 10];
    let _ = getrandom::getrandom(&mut buffer);
    buffer.to_vec()
}
