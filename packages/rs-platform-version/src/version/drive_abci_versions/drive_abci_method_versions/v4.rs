use crate::version::drive_abci_versions::drive_abci_method_versions::v3::DRIVE_ABCI_METHOD_VERSIONS_V3;
use crate::version::drive_abci_versions::drive_abci_method_versions::{
    DriveAbciBlockEndMethodVersions, DriveAbciMethodVersions,
};

pub const DRIVE_ABCI_METHOD_VERSIONS_V4: DriveAbciMethodVersions = DriveAbciMethodVersions {
    block_end: DriveAbciBlockEndMethodVersions {
        validator_set_update: 2, // Fixed rotation logic for the last element https://github.com/dashpay/platform/pull/2290
        ..DRIVE_ABCI_METHOD_VERSIONS_V3.block_end
    },
    ..DRIVE_ABCI_METHOD_VERSIONS_V3
};
