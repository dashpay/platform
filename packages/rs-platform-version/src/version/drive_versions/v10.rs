use crate::version::drive_versions::drive_token_method_versions::v3::DRIVE_TOKEN_METHOD_VERSIONS_V3;
use crate::version::drive_versions::v9::DRIVE_VERSION_V9;
use crate::version::drive_versions::{
    DriveInitializationMethodVersions, DriveMethodVersions, DriveVersion,
};

/// Drive version 10.
/// Introduced in protocol v15 for the token marketplace:
///
/// * **Escrow trees** — `create_initial_state_structure` 5 adds the two escrow trees of the
///   token marketplace: `[Tokens, TOKEN_ORDER_ESCROW_KEY]`, the BigSumTree of the tokens open
///   listings hold, and `[PreFundedSpecializedBalances, PREFUNDED_BALANCES_FOR_TOKEN_ORDERS]`,
///   the SumTree of the credits open offers hold. The upgrade path
///   (`Platform::transition_to_version_15`) inserts them through the same helper.
/// * **Token conservation** — `DRIVE_TOKEN_METHOD_VERSIONS_V3` bumps
///   `calculate_total_tokens_balance` to 2, which counts the escrowed listings on the balance
///   side.
///
/// Everything else matches `DRIVE_VERSION_V9`.
pub const DRIVE_VERSION_V10: DriveVersion = DriveVersion {
    methods: DriveMethodVersions {
        initialization: DriveInitializationMethodVersions {
            create_initial_state_structure: 5, // changed in v10: adds the token marketplace escrow trees
        },
        token: DRIVE_TOKEN_METHOD_VERSIONS_V3, // changed in v10: calculate_total_tokens_balance 2 counts the token marketplace escrow
        ..DRIVE_VERSION_V9.methods
    },
    ..DRIVE_VERSION_V9
};
