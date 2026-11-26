use super::v11::DRIVE_ABCI_METHOD_VERSIONS_V11;
use super::{DriveAbciBlockEndMethodVersions, DriveAbciMethodVersions};

/// Introduced in protocol version 16. Differs from v11 in:
/// - `block_end.apply_random_beacon` is `Some(0)`: a block collects the documents it creates
///   and its random token transfers and mints, and `ApplyBlockRandomness` (or `FinalizeBlock`,
///   with the commit's beacon) draws their values from the block's random beacon, which
///   Tenderdash sends from consensus version 3 on (v16 note 4).
pub const DRIVE_ABCI_METHOD_VERSIONS_V12: DriveAbciMethodVersions = DriveAbciMethodVersions {
    block_end: DriveAbciBlockEndMethodVersions {
        apply_random_beacon: Some(0), // changed: the block's random beacon is applied
        ..DRIVE_ABCI_METHOD_VERSIONS_V11.block_end
    },
    ..DRIVE_ABCI_METHOD_VERSIONS_V11
};
