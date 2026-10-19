use super::v10::DRIVE_ABCI_METHOD_VERSIONS_V10;
use super::{DriveAbciEngineMethodVersions, DriveAbciMethodVersions};

/// Protocol 15 installs the full evidence policy when it changes.
pub const DRIVE_ABCI_METHOD_VERSIONS_V11: DriveAbciMethodVersions = DriveAbciMethodVersions {
    engine: DriveAbciEngineMethodVersions {
        consensus_params_update: 2,
        ..DRIVE_ABCI_METHOD_VERSIONS_V10.engine
    },
    ..DRIVE_ABCI_METHOD_VERSIONS_V10
};
