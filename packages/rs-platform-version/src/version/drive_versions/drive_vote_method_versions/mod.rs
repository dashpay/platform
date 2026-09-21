use versioned_feature_core::FeatureVersion;

pub mod v1;
pub mod v2;
pub mod v3;

#[derive(Clone, Debug, Default)]
pub struct DriveVoteMethodVersions {
    pub insert: DriveVoteInsertMethodVersions,
    pub contested_resource_insert: DriveVoteContestedResourceInsertMethodVersions,
    pub cleanup: DriveVoteCleanupMethodVersions,
    pub setup: DriveVoteSetupMethodVersions,
    pub storage_form: DriveVoteStorageFormMethodVersions,
    pub fetch: DriveVoteFetchMethodVersions,
    /// Identity contender vote polls (protocol version 14).
    pub identity_contender: DriveVoteIdentityContenderMethodVersions,
}

/// The methods of identity contender vote polls: opening one, adding contenders, casting and
/// reading votes, and cleaning up when it ends.
#[derive(Clone, Debug, Default)]
pub struct DriveVoteIdentityContenderMethodVersions {
    pub open_identity_contender_vote_poll_operations: FeatureVersion,
    pub add_identity_contender_operations: FeatureVersion,
    pub register_identity_contender_vote_poll_identity_vote: FeatureVersion,
    pub insert_stored_info_for_identity_contender_vote_poll: FeatureVersion,
    pub fetch_identity_contender_vote_poll_stored_info: FeatureVersion,
    pub fetch_identity_contender_info: FeatureVersion,
    pub fetch_identity_contender_vote_poll_identity_vote: FeatureVersion,
    pub fetch_identity_contender_vote_poll_contenders: FeatureVersion,
    pub fetch_identities_voting_in_identity_contender_vote_poll: FeatureVersion,
    pub remove_identity_contender_vote_poll_operations: FeatureVersion,
    pub remove_identity_contender_vote_poll_end_date_query_operations: FeatureVersion,
    pub remove_identity_contender_vote_references_given_by_identity_operations: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVoteFetchMethodVersions {
    pub fetch_identities_voting_for_contenders: FeatureVersion,
    pub fetch_contested_document_vote_poll_stored_info: FeatureVersion,
    pub fetch_identity_contested_resource_vote: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVoteStorageFormMethodVersions {
    pub resolve_with_contract: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVoteSetupMethodVersions {
    pub add_initial_vote_tree_main_structure_operations: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVoteCleanupMethodVersions {
    /// Removing every vote of the masternodes that left the list. Version 1 (protocol version
    /// 14) also removes their votes on identity contender vote polls.
    pub remove_all_votes_given_by_identities: FeatureVersion,
    pub remove_specific_vote_references_given_by_identity: FeatureVersion,
    pub remove_specific_votes_given_by_identity: FeatureVersion,
    pub remove_contested_resource_vote_poll_end_date_query_operations: FeatureVersion,
    pub remove_contested_resource_vote_poll_votes_operations: FeatureVersion,
    pub remove_contested_resource_vote_poll_documents_operations: FeatureVersion,
    pub remove_contested_resource_vote_poll_contenders_operations: FeatureVersion,
    pub remove_contested_resource_top_level_index_operations: FeatureVersion,
    pub remove_contested_resource_info_operations: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVoteInsertMethodVersions {
    pub register_identity_vote: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVoteContestedResourceInsertMethodVersions {
    pub register_contested_resource_identity_vote: FeatureVersion,
    pub insert_stored_info_for_contested_resource_vote_poll: FeatureVersion,
    pub register_identity_vote: FeatureVersion,
    pub add_vote_poll_end_date_query_operations: FeatureVersion,
}
