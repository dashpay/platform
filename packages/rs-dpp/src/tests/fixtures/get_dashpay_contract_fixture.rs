use std::sync::Arc;

use crate::{
    data_contract::{DataContract, DataContractFactory},
    prelude::Identifier,
    tests::utils::generate_random_identifier_struct,
    version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION},
};

use crate::data_contract::CreatedDataContract;
use data_contracts::SystemDataContract;

pub fn get_dashpay_contract_fixture(owner_id: Option<Identifier>) -> CreatedDataContract {
    let protocol_version_validator =
        ProtocolVersionValidator::new(LATEST_VERSION, LATEST_VERSION, COMPATIBILITY_MAP.clone());
    let factory = DataContractFactory::new(1, 0, None);
    let dpns_schema = SystemDataContract::Dashpay
        .source()
        .expect("DPNS contract must be defined")
        .document_schemas;
    let owner_id = owner_id.unwrap_or_else(generate_random_identifier_struct);

    factory
        .create(owner_id, dpns_schema.into(), None, None)
        .expect("data in fixture should be correct")
}
