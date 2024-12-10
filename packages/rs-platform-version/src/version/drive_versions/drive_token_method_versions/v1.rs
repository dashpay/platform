use crate::version::drive_versions::drive_token_method_versions::{
    DriveTokenFetchMethodVersions, DriveTokenInsertMethodVersions, DriveTokenMethodVersions,
    DriveTokenProveMethodVersions,
};

pub const DRIVE_TOKEN_METHOD_VERSIONS_V1: DriveTokenMethodVersions = DriveTokenMethodVersions {
    fetch: DriveTokenFetchMethodVersions {},
    prove: DriveTokenProveMethodVersions {},
    insert: DriveTokenInsertMethodVersions {
        create_token_root_tree: 0,
    },
};
