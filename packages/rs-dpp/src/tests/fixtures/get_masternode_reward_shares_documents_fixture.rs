use serde_json::json;

use crate::{
    document::{document_factory::DocumentFactory, Document},
    mocks,
    tests::utils::generate_random_identifier_struct,
    version::LATEST_VERSION,
};

use super::{get_document_validator_fixture, get_master_node_reward_shares_contract_fixture};

pub fn get_masternode_reward_shares_documents_fixture() -> Document {
    let owner_id = generate_random_identifier_struct();
    let pay_to_id = generate_random_identifier_struct();
    let data_contract = get_master_node_reward_shares_contract_fixture();

    let document_validator = get_document_validator_fixture();
    let factory = DocumentFactory::new(
        LATEST_VERSION,
        document_validator,
        mocks::FetchAndValidateDataContract {},
    );

    factory
        .create(
            data_contract,
            owner_id,
            String::from("rewardShare"),
            json!({
                "payToId": pay_to_id.as_bytes(),
                "percentage" : 500,

            }),
        )
        .expect("document for masternode reward shares contract should be created")
}
