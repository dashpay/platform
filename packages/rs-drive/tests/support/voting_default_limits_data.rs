//! Shared voting inputs that require only DPP, without a storage engine.

use dpp::data_contract::{
    accessors::v0::DataContractV0Getters, document_type::accessors::DocumentTypeV0Getters,
};
use dpp::platform_value::Value;
use dpp::prelude::{DataContract, Identifier};
use dpp::tests::fixtures::get_dpns_data_contract_fixture;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::{
    contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll, VotePoll,
};

pub const VOTER: [u8; 32] = [4; 32];
pub const END_TIME: u64 = 1_000_000;
pub const PROTOCOL_VERSION: u32 = 14;

/// Committed responses use this protocol rather than following the latest release.
pub fn version() -> &'static PlatformVersion {
    PlatformVersion::get(PROTOCOL_VERSION).expect("fixture protocol version")
}

pub fn contract(platform_version: &PlatformVersion) -> DataContract {
    get_dpns_data_contract_fixture(
        Some(Identifier::from([5; 32])),
        1,
        platform_version.protocol_version,
    )
    .data_contract_owned()
}

/// Reconstructs the concrete poll input used to populate the fixture.
pub fn poll(contract: &DataContract, i: usize) -> VotePoll {
    ContestedDocumentResourceVotePoll {
        contract_id: contract.id(),
        document_type_name: "domain".into(),
        index_name: contract
            .document_type_for_name("domain")
            .expect("domain")
            .find_contested_index()
            .expect("contested index")
            .name
            .clone(),
        index_values: vec![
            Value::Text("dash".into()),
            Value::Text(format!("name{i:03}")),
        ],
    }
    .into()
}
