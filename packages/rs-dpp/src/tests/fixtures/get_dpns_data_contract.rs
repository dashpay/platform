use std::sync::Arc;

use lazy_static::lazy_static;
use serde_json::{json, Value};

use crate::prelude::*;
use crate::{
    data_contract::validation::data_contract_validator::DataContractValidator,
    data_contract::DataContractFactory,
    identifier,
    tests::utils::generate_random_identifier_struct,
    version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION},
};

lazy_static! {
    static ref DPNS_SCHEMA: Value = serde_json::from_str(include_str!(
        "./../../../contracts/dpns/dpns-contract-documents.json"
    ))
    .expect("dpns contract should be valid json");
}

pub fn get_dpns_data_contract_fixture(owner_id: Option<Identifier>) -> DataContract {
    let protocol_version_validator =
        ProtocolVersionValidator::new(LATEST_VERSION, LATEST_VERSION, COMPATIBILITY_MAP.clone());
    let data_contract_validator = DataContractValidator::new(Arc::new(protocol_version_validator));
    let factory = DataContractFactory::new(1, data_contract_validator);

    let owner_id = owner_id.unwrap_or_else(generate_random_identifier_struct);

    let mut dpns_schema = DPNS_SCHEMA.clone();
    // TODO the pattern is invalid as it's a re2
    dpns_schema["domain"]["properties"]["normalizedParentDomainName"]["pattern"] = json!(".*");

    let mut data_contract = factory
        .create(owner_id, dpns_schema)
        .expect("data in fixture should be correct");

    data_contract
        .defs
        .insert(String::from("lastName"), json!({ "type" : "string"}));

    data_contract
}
