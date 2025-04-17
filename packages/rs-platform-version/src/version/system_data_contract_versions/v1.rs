use crate::version::system_data_contract_versions::SystemDataContractVersions;

pub const SYSTEM_DATA_CONTRACT_VERSIONS_V1: SystemDataContractVersions =
    SystemDataContractVersions {
        withdrawals: 1,
        dpns: 1,
        dashpay: 1,
        masternode_reward_shares: 1,
        feature_flags: 1,
        wallet: 1,
        token_history: 1,
        keyword_search: 1,
    };
