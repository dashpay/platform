use std::sync::Arc;

use json_patch::merge;
use serde_json::{json, Value};

use crate::{
    document::{
        document_factory::DocumentFactory,
        fetch_and_validate_data_contract::DataContractFetcherAndValidator, Document,
    },
    prelude::Identifier,
    state_repository::MockStateRepositoryLike,
    tests::utils::generate_random_identifier_struct,
    version::LATEST_VERSION,
};

use super::{get_dashpay_contract_fixture, get_document_validator_fixture};

pub fn get_contact_request_document_fixture(
    owner_id: Option<Identifier>,
    additional_data: Option<Value>,
) -> Document {
    let owner_id = owner_id.unwrap_or_else(generate_random_identifier_struct);
    let data_contract = get_dashpay_contract_fixture(None);

    let data_contract_fetcher_and_validator =
        DataContractFetcherAndValidator::new(Arc::new(MockStateRepositoryLike::new()));
    let factory = DocumentFactory::new(
        LATEST_VERSION,
        get_document_validator_fixture(),
        data_contract_fetcher_and_validator,
    );

    let mut data = json! ({
            "toUserId": vec![0_u8;32],
            "encryptedPublicKey": vec![0_u8;96],
            "senderKeyIndex": 0,
            "recipientKeyIndex": 0,
            "accountReference": 0,
    });

    if let Some(additional_data) = additional_data {
        merge(&mut data, &additional_data)
    }

    factory
        .create(data_contract, owner_id, "contactRequest".to_string(), data)
        .expect("the document dashpay contact request should be created")
}
