use std::sync::Arc;

use data_contracts::SystemDataContract;
use platform_value::platform_value;

use crate::document::ExtendedDocument;
use crate::system_data_contracts::load_system_data_contract;
use crate::{
    data_contract::DataContract,
    document::{
        document_factory::DocumentFactory,
        fetch_and_validate_data_contract::DataContractFetcherAndValidator,
    },
    state_repository::MockStateRepositoryLike,
    tests::utils::generate_random_identifier_struct,
    version::LATEST_VERSION,
};

use super::get_document_validator_fixture;

pub fn get_masternode_reward_shares_documents_fixture() -> (Vec<ExtendedDocument>, DataContract) {
    let owner_id = generate_random_identifier_struct();
    let pay_to_id = generate_random_identifier_struct();
    let data_contract = load_system_data_contract(SystemDataContract::MasternodeRewards)
        .expect("should load masternode rewards contract");

    let document_validator = get_document_validator_fixture();
    let factory = DocumentFactory::new(
        LATEST_VERSION,
        document_validator,
        DataContractFetcherAndValidator::new(Arc::new(MockStateRepositoryLike::new())),
        None,
    );

    (
        vec![factory
            .create_extended_document_for_state_transition(
                data_contract.clone(),
                owner_id,
                String::from("rewardShare"),
                platform_value!({
                    "payToId": pay_to_id,
                    "percentage" : 500u16,
                }),
            )
            .expect("document for masternode reward shares contract should be created")],
        data_contract,
    )
}
