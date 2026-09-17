use crate::version::drive_abci_versions::drive_abci_method_versions::v10::DRIVE_ABCI_METHOD_VERSIONS_V10;
use crate::version::drive_abci_versions::drive_abci_method_versions::{
    DriveAbciMethodVersions, DriveAbciProtocolUpgradeMethodVersions,
};

/// Drive ABCI method versions 11. Introduced in protocol version 17 (5.0).
///
/// One slot changes over `DRIVE_ABCI_METHOD_VERSIONS_V10`:
///
/// * `protocol_upgrade.perform_events_on_first_block_of_protocol_change` becomes `Some(2)`:
///   the generation that runs the transition to protocol version 17 (the token contract
///   lifecycle ledger built through the genesis helper and one record per issuer backfilled
///   from the supply tree) when the chain crosses it, on top of everything generation 1 does.
///
/// Everything else matches v10.
pub const DRIVE_ABCI_METHOD_VERSIONS_V11: DriveAbciMethodVersions = DriveAbciMethodVersions {
    protocol_upgrade: DriveAbciProtocolUpgradeMethodVersions {
        perform_events_on_first_block_of_protocol_change: Some(2), // changed in v17: runs the token lifecycle ledger transition
        ..DRIVE_ABCI_METHOD_VERSIONS_V10.protocol_upgrade
    },
    ..DRIVE_ABCI_METHOD_VERSIONS_V10
};
