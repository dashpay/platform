use std::sync::Arc;

use crate::document::Document;
#[cfg(feature = "extended-document")]
use crate::document::ExtendedDocument;
use crate::system_data_contracts::load_system_data_contract;
use crate::{
    data_contract::DataContract, document::document_factory::DocumentFactory,
    tests::utils::generate_random_identifier_struct,
};
use data_contracts::SystemDataContract;
use platform_value::platform_value;

pub fn get_masternode_reward_shares_documents_fixture(
    protocol_version: u32,
) -> (Vec<Document>, DataContract) {
    let owner_id = generate_random_identifier_struct();
    let pay_to_id = generate_random_identifier_struct();
    let data_contract =
        load_system_data_contract(SystemDataContract::MasternodeRewards, protocol_version)
            .expect("should load masternode rewards contract");

    let factory = DocumentFactory::new(protocol_version, data_contract.clone())
        .expect("expected to make factory");

    (
        vec![factory
            .create_document(
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

pub fn get_masternode_reward_shares_data_contract_fixture(protocol_version: u32) -> DataContract {
    load_system_data_contract(SystemDataContract::MasternodeRewards, protocol_version)
        .expect("should load masternode rewards contract")
}

#[cfg(feature = "extended-document")]
pub fn get_masternode_reward_shares_extended_documents_fixture(
    protocol_version: u32,
) -> (Vec<ExtendedDocument>, DataContract) {
    let owner_id = generate_random_identifier_struct();
    let pay_to_id = generate_random_identifier_struct();
    let data_contract =
        load_system_data_contract(SystemDataContract::MasternodeRewards, protocol_version)
            .expect("should load masternode rewards contract");

    let factory = DocumentFactory::new(protocol_version, data_contract.clone())
        .expect("expected to make factory");

    (
        vec![factory
            .create_extended_document(
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
