use crate::version::drive_versions::drive_token_method_versions::{
    DriveTokenFetchMethodVersions, DriveTokenMethodVersions, DriveTokenProveMethodVersions,
    DriveTokenUpdateMethodVersions,
};

pub const DRIVE_TOKEN_METHOD_VERSIONS_V1: DriveTokenMethodVersions = DriveTokenMethodVersions {
    fetch: DriveTokenFetchMethodVersions { balance: 0 },
    prove: DriveTokenProveMethodVersions {},
    update: DriveTokenUpdateMethodVersions {
        create_token_root_tree: 0,
        burn: 0,
        mint: 0,
        transfer: 0,
        add_to_token_total_supply: 0,
        remove_from_token_total_supply: 0,
        remove_from_identity_token_balance: 0,
        add_to_identity_token_balance: 0,
    },
};
