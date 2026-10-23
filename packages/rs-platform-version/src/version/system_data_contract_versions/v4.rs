use crate::version::system_data_contract_versions::SystemDataContractVersions;

// PROTOCOL_VERSION_15: the DPNS contract v3 checks a domain create with schema
// keywords instead of most of its data trigger: generated normalized names, the
// reveal and deletion of the writer's own preorder from an earlier block
// (preorders unsalted, one per owner and name, the first valid one of a name
// chosen), and the new domain's identity record held to its owner. Domains can
// no longer be deleted. The token market contract (token_market: 1), the open
// listings and offers of the token marketplace, is written to state by the
// upgrade to 15 and registered at genesis from 15 on. The token history
// contract v3 records the marketplace's events (orderPlaced, orderFilled,
// orderCancelled) for a token that keeps transfer history. v3 (dpns: 2,
// token_history: 2, no token market) remains for PROTOCOL_VERSION_14 chain
// replay.
pub const SYSTEM_DATA_CONTRACT_VERSIONS_V4: SystemDataContractVersions =
    SystemDataContractVersions {
        withdrawals: 2,
        dpns: 3,
        dashpay: 2,
        masternode_reward_shares: 1,
        wallet: 1,
        token_history: 3,
        keyword_search: 1,
        document_history: 1,
        app_connect: 1,
        moderation_charters: 1,
        token_market: 1,
    };
