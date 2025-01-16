use crate::version::drive_versions::drive_token_method_versions::{
    DriveTokenFetchMethodVersions, DriveTokenMethodVersions, DriveTokenProveMethodVersions,
    DriveTokenUpdateMethodVersions,
};

pub const DRIVE_TOKEN_METHOD_VERSIONS_V1: DriveTokenMethodVersions = DriveTokenMethodVersions {
    fetch: DriveTokenFetchMethodVersions {
        identity_token_balance: 0,
        identity_token_balances: 0,
        identities_token_balances: 0,
        identity_token_info: 0,
        identity_token_infos: 0,
        identities_token_infos: 0,
        token_statuses: 0,
        token_status: 0,
    },
    prove: DriveTokenProveMethodVersions {
        identity_token_balance: 0,
        identity_token_balances: 0,
        identities_token_balances: 0,
        identity_token_info: 0,
        identity_token_infos: 0,
        identities_token_infos: 0,
        token_statuses: 0,
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
        freeze: 0,
        unfreeze: 0,
        apply_status: 0,
    },
    calculate_total_tokens_balance: 0,
};
