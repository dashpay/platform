use lazy_static::lazy_static;
use serde_json::Value as JsonValue;
use std::sync::Arc;

lazy_static! {
    static ref MASTERNODE_REWARD_SHARES_DOCUMENTS: JsonValue = serde_json::from_str(include_str!(
        "./../../../../masternode-reward-shares-contract/schema/masternode-reward-shares-documents.json"
    ))
    .unwrap();
}

use crate::{
    data_contract::{
        validation::data_contract_validator::DataContractValidator, DataContract,
        DataContractFactory,
    },
    tests::utils::generate_random_identifier_struct,
    version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION},
};

pub fn get_master_node_reward_shares_contract_fixture() -> DataContract {
    let protocol_version_validator =
        ProtocolVersionValidator::new(LATEST_VERSION, LATEST_VERSION, COMPATIBILITY_MAP.clone());
    let data_contract_validator = DataContractValidator::new(Arc::new(protocol_version_validator));

    let owner_id = generate_random_identifier_struct();
    let factory = DataContractFactory::new(1, data_contract_validator);

    factory
        .create(owner_id, MASTERNODE_REWARD_SHARES_DOCUMENTS.clone())
        .expect("the contract for masternode reward shares documents should be created")
}
