use crate::version::system_data_contract_versions::SystemDataContractVersions;

// PROTOCOL_VERSION_14: DashPay contract v2 adds the optional public payment
// address fields to the `profile` document type (`corePaymentAddress`,
// `platformPaymentAddress`) per DIP-33 plus the optional 43-byte Orchard
// `shieldedAddress`, and the withdrawals contract v2 admits
// the terminal FAILED (5) value of the `status` property, written for
// withdrawals whose asset unlock Core can never mine. v2 (dashpay: 1,
// withdrawals: 1) remains for PROTOCOL_VERSION_13 chain replay.
pub const SYSTEM_DATA_CONTRACT_VERSIONS_V3: SystemDataContractVersions =
    SystemDataContractVersions {
        withdrawals: 2,
        dpns: 2,
        dashpay: 2,
        masternode_reward_shares: 1,
        wallet: 1,
        token_history: 1,
        keyword_search: 1,
        document_history: 1,
    };
