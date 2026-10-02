use crate::version::drive_versions::drive_vote_method_versions::v4::DRIVE_VOTE_METHOD_VERSIONS_V4;
use crate::version::drive_versions::v9::DRIVE_VERSION_V9;
use crate::version::drive_versions::{DriveMethodVersions, DriveVersion};

/// Drive version 10.
/// Introduced in protocol v17, the 5.0 protocol version, for compilation readiness: the
/// rounds, reports, cursors and deadlines Drive keeps under `[Votes] / r` and the readiness
/// funds under `[PreFundedSpecializedBalances] / 129`.
///
/// * **Genesis and storage**: `DRIVE_VOTE_METHOD_VERSIONS_V4` bumps
///   `setup.add_initial_vote_tree_main_structure_operations` to 1 so genesis creates both
///   subtrees (the upgrade path creates the same elements with insert-if-not-exists), and
///   turns on the `readiness` method group.
/// * **Proofs**: the three readiness verifiers are generation 0 in every verify table, so the
///   verify table of `DRIVE_VERSION_V9` is kept as is.
///
/// Everything else matches `DRIVE_VERSION_V9`.
pub const DRIVE_VERSION_V10: DriveVersion = DriveVersion {
    methods: DriveMethodVersions {
        vote: DRIVE_VOTE_METHOD_VERSIONS_V4, // changed in v10: readiness subtrees at genesis and the readiness method group
        ..DRIVE_VERSION_V9.methods
    },
    ..DRIVE_VERSION_V9
};
