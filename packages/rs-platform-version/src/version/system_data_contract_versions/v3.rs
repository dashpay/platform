use crate::version::system_data_contract_versions::SystemDataContractVersions;

// PROTOCOL_VERSION_14: DashPay contract v2 adds the optional public payment
// address fields to the `profile` document type (`corePaymentAddress`,
// `platformPaymentAddress`) per DIP-33 plus the optional 43-byte Orchard
// `shieldedAddress`, and the withdrawals contract v2 admits
// the terminal FAILED (5) value of the `status` property, written for
// withdrawals whose asset unlock Core can never mine. The token history
// contract v2 admits the value 2 (OncePerIdentity) of the claim document's
// `distributionType`, written for once-per-identity distribution claims.
// v2 (dashpay: 1, withdrawals: 1, token_history: 1) remains for
// PROTOCOL_VERSION_13 chain replay.
//
// The app-connect contract (app_connect: 1) also activates with
// PROTOCOL_VERSION_14: it is registered at genesis from that version on and
// inserted by `transition_to_version_14` on chains upgrading from 13. The
// earlier tables carry 0, which no schema generation answers to, so the table
// itself refuses to load the contract before 14.
//
// The moderation charters contract (moderation_charters: 1) is registered at
// PROTOCOL_VERSION_14 too, but is not yet written to state at genesis or on
// upgrade: the election that a charter create opens does not exist yet, and the
// PR that adds it writes the contract to state.
pub const SYSTEM_DATA_CONTRACT_VERSIONS_V3: SystemDataContractVersions =
    SystemDataContractVersions {
        withdrawals: 2,
        dpns: 2,
        dashpay: 2,
        masternode_reward_shares: 1,
        wallet: 1,
        token_history: 2,
        keyword_search: 1,
        document_history: 1,
        app_connect: 1,
        moderation_charters: 1,
    };
