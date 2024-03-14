use crate::{
    data_contract::DataContractFactory, prelude::Identifier,
    tests::utils::generate_random_identifier_struct,
};

use crate::data_contract::config::v0::DataContractConfigV0;
use crate::data_contract::created_data_contract::CreatedDataContract;
use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
use crate::prelude::IdentityNonce;
use data_contracts::SystemDataContract;
use platform_version::version::PlatformVersion;

pub fn get_dashpay_contract_with_generalized_encryption_key_fixture(
    owner_id: Option<Identifier>,
    identity_nonce: IdentityNonce,
    protocol_version: u32,
) -> CreatedDataContract {
    let factory = DataContractFactory::new(protocol_version).expect("expected to create factory");

    let platform_version = PlatformVersion::get(protocol_version).expect("expected to get version");

    let dpns_schema = SystemDataContract::Dashpay
        .source(platform_version)
        .expect("DPNS contract must be defined")
        .document_schemas;
    let owner_id = owner_id.unwrap_or_else(generate_random_identifier_struct);

    factory
        .create(
            owner_id,
            identity_nonce,
            dpns_schema.into(),
            Some(
                DataContractConfigV0 {
                    requires_identity_encryption_bounded_key: Some(StorageKeyRequirements::Unique),
                    requires_identity_decryption_bounded_key: Some(StorageKeyRequirements::Unique),
                    ..Default::default()
                }
                .into(),
            ),
            None,
        )
        .expect("data in fixture should be correct")
}
