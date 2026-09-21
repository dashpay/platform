use super::v10::DRIVE_ABCI_METHOD_VERSIONS_V10;
use super::{DriveAbciBlockEndMethodVersions, DriveAbciMethodVersions};

/// Protocol 15 records and prunes the anchors of every token shielded pool touched by a block.
pub const DRIVE_ABCI_METHOD_VERSIONS_V11: DriveAbciMethodVersions = DriveAbciMethodVersions {
    block_end: DriveAbciBlockEndMethodVersions {
        record_token_shielded_pool_anchors: Some(0),
        ..DRIVE_ABCI_METHOD_VERSIONS_V10.block_end
    },
    ..DRIVE_ABCI_METHOD_VERSIONS_V10
};
