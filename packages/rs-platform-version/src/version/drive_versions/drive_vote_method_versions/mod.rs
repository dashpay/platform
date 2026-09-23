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
    /// Yes/no polls: the decision polls under the votes tree's `d` branch (protocol version 14).
    pub yes_no: DriveVoteYesNoMethodVersions,
}

/// The methods of the yes/no poll kind. All of them are new in protocol version 14 and
/// unreachable before it: no yes/no poll can be opened or voted on earlier.
#[derive(Clone, Debug, Default)]
pub struct DriveVoteYesNoMethodVersions {
    pub open_yes_no_vote_poll: FeatureVersion,
    pub register_yes_no_identity_vote: FeatureVersion,
    pub insert_stored_info_for_yes_no_vote_poll: FeatureVersion,
    pub fetch_yes_no_vote_poll_stored_info: FeatureVersion,
    pub fetch_identity_yes_no_vote: FeatureVersion,
    pub fetch_identities_voting_in_yes_no_vote_poll: FeatureVersion,
    pub remove_yes_no_vote_poll_votes_operations: FeatureVersion,
    pub remove_yes_no_vote_references_given_by_identity: FeatureVersion,
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
    pub remove_specific_vote_references_given_by_identity: FeatureVersion,
    pub remove_specific_votes_given_by_identity: FeatureVersion,
    pub remove_contested_resource_vote_poll_end_date_query_operations: FeatureVersion,
    pub remove_contested_resource_vote_poll_votes_operations: FeatureVersion,
    pub remove_contested_resource_vote_poll_documents_operations: FeatureVersion,
    pub remove_contested_resource_vote_poll_contenders_operations: FeatureVersion,
    pub remove_contested_resource_top_level_index_operations: FeatureVersion,
    pub remove_contested_resource_info_operations: FeatureVersion,
    /// Removes every vote a removed masternode cast. Version 1 (protocol version 14) also
    /// sweeps its yes/no votes.
    pub remove_all_votes_given_by_identities: FeatureVersion,
    /// Removes finished polls of every kind from the end date index (protocol version 14).
    pub remove_vote_poll_end_date_query_operations: FeatureVersion,
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
