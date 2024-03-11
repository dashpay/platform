use crate::{
    data_contract::DataContractFactory, prelude::Identifier,
    tests::utils::generate_random_identifier_struct,
};

use crate::data_contract::created_data_contract::CreatedDataContract;
use crate::prelude::IdentityNonce;
use data_contracts::SystemDataContract;
use platform_version::version::PlatformVersion;

pub fn get_dashpay_contract_fixture(
    owner_id: Option<Identifier>,
    identity_nonce: IdentityNonce,
    protocol_version: u32,
) -> CreatedDataContract {
    let factory = DataContractFactory::new(protocol_version).expect("expected to create factory");

    let platform_version = PlatformVersion::get(protocol_version).expect("expected to get version");

    let dashpay_schema = SystemDataContract::Dashpay
        .source(platform_version)
        .expect("DPNS contract must be defined")
        .document_schemas;
    let owner_id = owner_id.unwrap_or_else(generate_random_identifier_struct);

    factory
        .create_with_value_config(owner_id, identity_nonce, dashpay_schema.into(), None, None)
        .expect("data in fixture should be correct")
}
