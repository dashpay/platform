use crate::version::drive_versions::drive_vote_method_versions::v4::DRIVE_VOTE_METHOD_VERSIONS_V4;
use crate::version::drive_versions::v9::DRIVE_VERSION_V9;
use crate::version::drive_versions::{DriveMethodVersions, DriveVersion};

/// Drive version 10.
/// Introduced in protocol v15:
///
/// * **Yes/no vote polls**: `DRIVE_VOTE_METHOD_VERSIONS_V4` bumps
///   `insert.register_identity_vote` to 1, which registers a vote of either poll kind, and
///   `cleanup.remove_all_votes_given_by_identities` to 1, which also removes the yes/no votes of
///   a masternode that left the list. It sets `yes_no.open_yes_no_vote_poll` to `Some(0)`, so
///   yes/no polls can be opened (`None`, not active, in every earlier table), and bumps
///   `contested_resource_insert.add_vote_poll_end_date_query_operations` to 1, which adds only
///   the entry when the batch already queues the end date's tree with other storage flags.
///
/// Everything else matches `DRIVE_VERSION_V9`.
pub const DRIVE_VERSION_V10: DriveVersion = DriveVersion {
    methods: DriveMethodVersions {
        vote: DRIVE_VOTE_METHOD_VERSIONS_V4, // changed in v10: the yes/no poll kind
        ..DRIVE_VERSION_V9.methods
    },
    ..DRIVE_VERSION_V9
};
