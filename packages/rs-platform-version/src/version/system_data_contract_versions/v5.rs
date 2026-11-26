use crate::version::system_data_contract_versions::v4::SYSTEM_DATA_CONTRACT_VERSIONS_V4;
use crate::version::system_data_contract_versions::SystemDataContractVersions;

// PROTOCOL_VERSION_16: the token market contract v1 (token_market: 1), whose `ask` and `bid`
// types hold the open orders of the token marketplace. It is registered at genesis from
// protocol version 16 and written to state, without storage flags, by the upgrade to it.
// Every other contract is as `SYSTEM_DATA_CONTRACT_VERSIONS_V4` has it.
pub const SYSTEM_DATA_CONTRACT_VERSIONS_V5: SystemDataContractVersions =
    SystemDataContractVersions {
        token_market: 1,
        ..SYSTEM_DATA_CONTRACT_VERSIONS_V4
    };
