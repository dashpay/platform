use std::sync::Arc;

use lazy_static::lazy_static;
use serde_json::Value;

use crate::{
    data_contract::{
        validation::data_contract_validator::DataContractValidator, DataContract,
        DataContractFactory,
    },
    prelude::Identifier,
    tests::utils::generate_random_identifier_struct,
    version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION},
};

lazy_static! {
    static ref DASHPAY_SCHEMA: Value = serde_json::from_str(include_str!(
        "./../../../../dashpay-contract/schema/dashpay.schema.json"
    ))
    .unwrap();
}

pub fn get_dashpay_contract_fixture(owner_id: Option<Identifier>) -> DataContract {
    let protocol_version_validator =
        ProtocolVersionValidator::new(LATEST_VERSION, LATEST_VERSION, COMPATIBILITY_MAP.clone());
    let data_contract_validator = DataContractValidator::new(Arc::new(protocol_version_validator));
    let factory = DataContractFactory::new(1, Arc::new(data_contract_validator));

    let owner_id = owner_id.unwrap_or_else(generate_random_identifier_struct);
    factory
        .create(owner_id, DASHPAY_SCHEMA.clone(), None)
        .expect("data in fixture should be correct")
}
