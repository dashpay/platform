use data_contracts::{DataContractSource, SystemDataContract};
use platform_version::version::PlatformVersion;

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

    let platform_version =
        PlatformVersion::get(protocol_version).expect("expected to get platform version");

    let DataContractSource {
        document_schemas, ..
    } = SystemDataContract::DPNS
        .source(platform_version)
        .expect("should return DPNS data contract source");

    //Todo create config
    factory
        .create_with_value_config(owner_id, document_schemas.into(), None, None)
        .expect("data in fixture should be correct")
}
