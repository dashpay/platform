use crate::version::system_data_contract_versions::SystemDataContractVersions;

// PROTOCOL_VERSION_14: DashPay contract v2 adds the optional public payment
// address fields to the `profile` document type (`corePaymentAddress`,
// `platformPaymentAddress`) per DIP-33. v2 (dashpay: 1) remains for
// PROTOCOL_VERSION_13 chain replay.
pub const SYSTEM_DATA_CONTRACT_VERSIONS_V3: SystemDataContractVersions =
    SystemDataContractVersions {
        withdrawals: 1,
        dpns: 2,
        dashpay: 2,
        masternode_reward_shares: 1,
        wallet: 1,
        token_history: 1,
        keyword_search: 1,
        document_history: 1,
    };
