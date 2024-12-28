use crate::version::drive_versions::drive_token_method_versions::{
    DriveTokenFetchMethodVersions, DriveTokenMethodVersions, DriveTokenProveMethodVersions,
    DriveTokenUpdateMethodVersions,
};

pub const DRIVE_TOKEN_METHOD_VERSIONS_V1: DriveTokenMethodVersions = DriveTokenMethodVersions {
    fetch: DriveTokenFetchMethodVersions {
        identity_token_balance: 0,
        identity_token_balances: 0,
    },
    prove: DriveTokenProveMethodVersions {
        identity_token_balance: 0,
        identity_token_balances: 0,
    },
    update: DriveTokenUpdateMethodVersions {
        create_token_trees: 0,
        burn: 0,
        mint: 0,
        transfer: 0,
        add_to_token_total_supply: 0,
        remove_from_token_total_supply: 0,
        remove_from_identity_token_balance: 0,
        add_to_identity_token_balance: 0,
        add_transaction_history_operations: 0,
    },
};
