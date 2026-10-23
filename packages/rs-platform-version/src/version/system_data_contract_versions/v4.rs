use crate::version::system_data_contract_versions::SystemDataContractVersions;

// PROTOCOL_VERSION_15: the DPNS contract v3 checks a domain create with schema
// keywords instead of most of its data trigger: generated normalized names, the
// reveal and deletion of the writer's own preorder from an earlier block
// (preorders unsalted, one per owner and name, the first valid one of a name
// chosen), and the new domain's identity record held to its owner. Domains can
// no longer be deleted. v3 (dpns: 2) remains for PROTOCOL_VERSION_14 chain
// replay.
pub const SYSTEM_DATA_CONTRACT_VERSIONS_V4: SystemDataContractVersions =
    SystemDataContractVersions {
        withdrawals: 2,
        dpns: 3,
        dashpay: 2,
        masternode_reward_shares: 1,
        wallet: 1,
        token_history: 2,
        keyword_search: 1,
        document_history: 1,
        app_connect: 1,
        moderation_charters: 1,
    };
