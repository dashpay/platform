use std::sync::Arc;

use data_contracts::{DataContractSource, SystemDataContract};
use platform_value::platform_value;
use serde_json::json;

use crate::prelude::*;
use crate::{
    data_contract::validation::data_contract_validation::DataContractValidator,
    data_contract::DataContractFactory,
    tests::utils::generate_random_identifier_struct,
    version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION},
};

pub fn get_dpns_data_contract_fixture(owner_id: Option<Identifier>) -> DataContract {
    let protocol_version_validator =
        ProtocolVersionValidator::new(LATEST_VERSION, LATEST_VERSION, COMPATIBILITY_MAP.clone());
    let data_contract_validator = DataContractValidator::new(Arc::new(protocol_version_validator));
    let factory = DataContractFactory::new(1, Arc::new(data_contract_validator));

    let owner_id = owner_id.unwrap_or_else(generate_random_identifier_struct);

    let DataContractSource {
        mut document_schemas,
        ..
    } = SystemDataContract::DPNS
        .source()
        .expect("should return DPNS data contract source");

    let defs = platform_value!({
        "lastName": { "type" : "string"},
    });

    // TODO the pattern is invalid as it's a re2
    document_schemas["domain"]["properties"]["normalizedParentDomainName"]["pattern"] = json!(".*");

    //todo: the config should not be None
    factory
        .create(owner_id, document_schemas.into(), None, Some(defs))
        .expect("data in fixture should be correct")
}
