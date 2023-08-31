use data_contracts::{DataContractSource, SystemDataContract};
use platform_value::platform_value;
use serde_json::json;

use crate::data_contract::created_data_contract::CreatedDataContract;
use crate::data_contract::DataContractFactory;
use crate::prelude::*;
use crate::tests::utils::generate_random_identifier_struct;

pub fn get_dpns_data_contract_fixture(
    owner_id: Option<Identifier>,
    protocol_version: u32,
) -> CreatedDataContract {
    let factory = DataContractFactory::new(protocol_version, None)
        .expect("expected to create a factory for get_dpns_data_contract_fixture");

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

    //Todo create config
    factory
        .create(owner_id, document_schemas.into(), None, Some(defs))
        .expect("data in fixture should be correct")
}
