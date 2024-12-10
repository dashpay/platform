use crate::version::drive_versions::drive_token_method_versions::{
    DriveTokenFetchMethodVersions, DriveTokenMethodVersions, DriveTokenProveMethodVersions,
    DriveTokenUpdateMethodVersions,
};

pub const DRIVE_TOKEN_METHOD_VERSIONS_V1: DriveTokenMethodVersions = DriveTokenMethodVersions {
    fetch: DriveTokenFetchMethodVersions {},
    prove: DriveTokenProveMethodVersions {},
    update: DriveTokenUpdateMethodVersions {
        create_token_root_tree: 0,
    },
};
