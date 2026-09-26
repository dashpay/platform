use crate::version::drive_versions::drive_vote_method_versions::{
    DriveVoteCleanupMethodVersions, DriveVoteContestedResourceInsertMethodVersions,
    DriveVoteFetchMethodVersions, DriveVoteInsertMethodVersions, DriveVoteMethodVersions,
    DriveVoteSetupMethodVersions, DriveVoteStorageFormMethodVersions,
};

/// Drive vote methods for protocol v14+.
///
/// Identical to [`super::v2::DRIVE_VOTE_METHOD_VERSIONS_V2`] except
/// `cleanup.remove_contested_resource_vote_poll_end_date_query_operations` is
/// bumped to `2`: the end-date cleanup of ended contested vote polls removes an
/// end date only once none of its polls remain.
pub const DRIVE_VOTE_METHOD_VERSIONS_V3: DriveVoteMethodVersions = DriveVoteMethodVersions {
    insert: DriveVoteInsertMethodVersions {
        register_identity_vote: 0,
    },
    contested_resource_insert: DriveVoteContestedResourceInsertMethodVersions {
        register_contested_resource_identity_vote: 0,
        insert_stored_info_for_contested_resource_vote_poll: 0,
        register_identity_vote: 0,
        add_vote_poll_end_date_query_operations: 0,
    },
    cleanup: DriveVoteCleanupMethodVersions {
        remove_specific_vote_references_given_by_identity: 0,
        remove_specific_votes_given_by_identity: 0,
        remove_contested_resource_vote_poll_end_date_query_operations: 2, // changed in v3
        remove_contested_resource_vote_poll_votes_operations: 0,
        remove_contested_resource_vote_poll_documents_operations: 1,
        remove_contested_resource_vote_poll_contenders_operations: 1,
        remove_contested_resource_top_level_index_operations: 0,
        remove_contested_resource_info_operations: 0,
    },
    setup: DriveVoteSetupMethodVersions {
        add_initial_vote_tree_main_structure_operations: 0,
    },
    storage_form: DriveVoteStorageFormMethodVersions {
        resolve_with_contract: 0,
    },
    fetch: DriveVoteFetchMethodVersions {
        fetch_identities_voting_for_contenders: 0,
        fetch_contested_document_vote_poll_stored_info: 0,
        fetch_identity_contested_resource_vote: 0,
    },
};
