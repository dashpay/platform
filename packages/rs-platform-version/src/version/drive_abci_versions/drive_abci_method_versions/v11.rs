use super::v10::DRIVE_ABCI_METHOD_VERSIONS_V10;
use super::{DriveAbciMethodVersions, DriveAbciProtocolUpgradeMethodVersions};

/// Drive ABCI method versions 11, introduced in protocol v15.
///
/// The protocol-change hook migrates retained document histories after
/// refreshing contracts. All other methods match v10.
pub const DRIVE_ABCI_METHOD_VERSIONS_V11: DriveAbciMethodVersions = DriveAbciMethodVersions {
    protocol_upgrade: DriveAbciProtocolUpgradeMethodVersions {
        perform_events_on_first_block_of_protocol_change: Some(2),
        ..DRIVE_ABCI_METHOD_VERSIONS_V10.protocol_upgrade
    },
    ..DRIVE_ABCI_METHOD_VERSIONS_V10
};
