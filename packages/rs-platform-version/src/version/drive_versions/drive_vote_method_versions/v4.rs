use crate::version::drive_versions::drive_vote_method_versions::v3::DRIVE_VOTE_METHOD_VERSIONS_V3;
use crate::version::drive_versions::drive_vote_method_versions::{
    DriveVoteCleanupMethodVersions, DriveVoteInsertMethodVersions, DriveVoteMethodVersions,
    DriveVoteYesNoMethodVersions,
};

/// Drive vote method versions 4. Introduced in protocol version 15 with the yes/no poll
/// kind:
///
/// * `insert.register_identity_vote` 1 registers a resolved vote of either kind (v0 knows only
///   contested resource votes, which v1 still hands to it).
/// * `remove_all_votes_given_by_identities` 1 also removes the yes/no votes of a masternode
///   that left the list.
/// * `yes_no.open_yes_no_vote_poll` is `Some(0)`: yes/no polls can be opened from this version
///   (`None` in every earlier table).
///
/// Everything else matches `DRIVE_VOTE_METHOD_VERSIONS_V3`.
pub const DRIVE_VOTE_METHOD_VERSIONS_V4: DriveVoteMethodVersions = DriveVoteMethodVersions {
    insert: DriveVoteInsertMethodVersions {
        register_identity_vote: 1,
    },
    cleanup: DriveVoteCleanupMethodVersions {
        remove_all_votes_given_by_identities: 1,
        ..DRIVE_VOTE_METHOD_VERSIONS_V3.cleanup
    },
    yes_no: DriveVoteYesNoMethodVersions {
        open_yes_no_vote_poll: Some(0),
        ..DRIVE_VOTE_METHOD_VERSIONS_V3.yes_no
    },
    ..DRIVE_VOTE_METHOD_VERSIONS_V3
};
