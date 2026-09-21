use super::drive_contract_method_versions::v5::DRIVE_CONTRACT_METHOD_VERSIONS_V5;
use super::drive_token_method_versions::v3::DRIVE_TOKEN_METHOD_VERSIONS_V3;
use super::v9::DRIVE_VERSION_V9;
use super::{DriveInitializationMethodVersions, DriveMethodVersions, DriveVersion};

/// Protocol 15 extends protocol 14 storage with token shielded pools and their conservation check.
pub const DRIVE_VERSION_V10: DriveVersion = DriveVersion {
    methods: DriveMethodVersions {
        initialization: DriveInitializationMethodVersions {
            create_initial_state_structure: 5,
        },
        token: DRIVE_TOKEN_METHOD_VERSIONS_V3,
        contract: DRIVE_CONTRACT_METHOD_VERSIONS_V5,
        ..DRIVE_VERSION_V9.methods
    },
    ..DRIVE_VERSION_V9
};
