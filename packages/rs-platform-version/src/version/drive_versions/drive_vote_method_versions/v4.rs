use crate::version::drive_versions::drive_vote_method_versions::v3::DRIVE_VOTE_METHOD_VERSIONS_V3;
use crate::version::drive_versions::drive_vote_method_versions::{
    DriveVoteCleanupMethodVersions, DriveVoteContestedResourceInsertMethodVersions,
    DriveVoteInsertMethodVersions, DriveVoteMethodVersions, DriveVoteYesNoMethodVersions,
};

/// Drive vote method versions 4. Introduced in protocol version 15 with the yes/no poll
/// kind:
///
/// * `insert.register_identity_vote` 1 registers a resolved vote of either kind (v0 knows only
///   contested resource votes, which v1 still hands to it).
/// * `remove_all_votes_given_by_identities` 1 also removes the yes/no votes of a masternode
///   that left the list.
/// * `cleanup.remove_vote_poll_end_date_query_operations` is `Some(0)`: the yes/no clean-up
///   removes finished polls from the end date index with it (`None`, not active, before).
/// * every `yes_no` method is `Some(0)`: yes/no polls can be opened and voted on from this
///   version (`None`, not active, in every earlier table).
/// * `contested_resource_insert.add_vote_poll_end_date_query_operations` 1 adds only the entry
///   when the batch already queues an insert of the end date's tree with other storage flags (a
///   contest and a yes/no poll opening sharing an end date), where 0 queued the tree twice.
///
/// Everything else matches `DRIVE_VOTE_METHOD_VERSIONS_V3`.
pub const DRIVE_VOTE_METHOD_VERSIONS_V4: DriveVoteMethodVersions = DriveVoteMethodVersions {
    insert: DriveVoteInsertMethodVersions {
        register_identity_vote: 1,
    },
    contested_resource_insert: DriveVoteContestedResourceInsertMethodVersions {
        add_vote_poll_end_date_query_operations: 1,
        ..DRIVE_VOTE_METHOD_VERSIONS_V3.contested_resource_insert
    },
    cleanup: DriveVoteCleanupMethodVersions {
        remove_all_votes_given_by_identities: 1,
        remove_vote_poll_end_date_query_operations: Some(0),
        ..DRIVE_VOTE_METHOD_VERSIONS_V3.cleanup
    },
    yes_no: DriveVoteYesNoMethodVersions {
        open_yes_no_vote_poll: Some(0),
        register_yes_no_identity_vote: Some(0),
        insert_stored_info_for_yes_no_vote_poll: Some(0),
        fetch_yes_no_vote_poll_stored_info: Some(0),
        fetch_identity_yes_no_vote: Some(0),
        fetch_identities_voting_in_yes_no_vote_poll: Some(0),
        remove_yes_no_vote_poll_votes_operations: Some(0),
        remove_yes_no_vote_references_given_by_identity: Some(0),
    },
    ..DRIVE_VOTE_METHOD_VERSIONS_V3
};
