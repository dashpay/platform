use crate::version::system_data_contract_versions::SystemDataContractVersions;

// PROTOCOL_VERSION_13: DPNS contract v2 subscribes the `domain` document type
// to the document history system contract (keepsTransferHistory,
// keepsPurchaseHistory and keepsPricingHistory), so username transfers, sales
// and listings are recorded and queryable. v1 (dpns: 1) remains for
// PROTOCOL_VERSION_12 chain replay.
pub const SYSTEM_DATA_CONTRACT_VERSIONS_V2: SystemDataContractVersions =
    SystemDataContractVersions {
        withdrawals: 1,
        dpns: 2,
        dashpay: 1,
        masternode_reward_shares: 1,
        wallet: 1,
        token_history: 1,
        keyword_search: 1,
        document_history: 1,
        // The app-connect contract does not exist before protocol version 14: no
        // schema generation is selected here, so loading it is refused.
        app_connect: 0,
        // The moderation charters contract does not exist before protocol version 14
        // either: no schema generation is selected here.
        moderation_charters: 0,
    };
