use std::sync::Arc;

use lazy_static::lazy_static;
use serde_json::Value;

use crate::contracts::withdrawals_contract;
use crate::prelude::*;
use crate::util::string_encoding::Encoding;
use crate::{
    data_contract::validation::data_contract_validator::DataContractValidator,
    data_contract::DataContractFactory,
    tests::utils::generate_random_identifier_struct,
    version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION},
};

lazy_static! {
    static ref WITHDRAWALS_SCHEMA: Value = serde_json::from_str(include_str!(
        "./../../../contracts/withdrawals/withdrawals-contract-documents.json"
    ))
    .expect("withdrawals contract should be valid json");
}

pub fn get_withdrawals_data_contract_fixture(owner_id: Option<Identifier>) -> DataContract {
    let protocol_version_validator =
        ProtocolVersionValidator::new(LATEST_VERSION, LATEST_VERSION, COMPATIBILITY_MAP.clone());
    let data_contract_validator = DataContractValidator::new(Arc::new(protocol_version_validator));
    let factory = DataContractFactory::new(1, data_contract_validator);

    let owner_id = owner_id.unwrap_or_else(generate_random_identifier_struct);

    let withdrawals_schema = WITHDRAWALS_SCHEMA.clone();

    let mut data_contract = factory
        .create(owner_id, withdrawals_schema)
        .expect("data in fixture should be correct");

    data_contract.id = Identifier::from_string(
        &withdrawals_contract::system_ids().contract_id,
        Encoding::Base58,
    )
    .unwrap();

    data_contract
}
