use versioned_feature_core::{FeatureVersion, OptionalFeatureVersion};

pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;

#[derive(Clone, Debug, Default)]
pub struct DriveVoteMethodVersions {
    pub insert: DriveVoteInsertMethodVersions,
    pub contested_resource_insert: DriveVoteContestedResourceInsertMethodVersions,
    pub cleanup: DriveVoteCleanupMethodVersions,
    pub setup: DriveVoteSetupMethodVersions,
    pub storage_form: DriveVoteStorageFormMethodVersions,
    pub fetch: DriveVoteFetchMethodVersions,
    /// Yes/no polls: the decision polls under the votes tree's `d` branch (protocol version 15).
    pub yes_no: DriveVoteYesNoMethodVersions,
}

/// The methods of the yes/no poll kind. All of them are new in protocol version 15 and `None`
/// (not active) in every earlier table: opening a poll is not active before 15, and a vote
/// naming a yes/no poll does not decode before 15.
#[derive(Clone, Debug, Default)]
pub struct DriveVoteYesNoMethodVersions {
    pub open_yes_no_vote_poll: OptionalFeatureVersion,
    pub register_yes_no_identity_vote: OptionalFeatureVersion,
    pub insert_stored_info_for_yes_no_vote_poll: OptionalFeatureVersion,
    pub fetch_yes_no_vote_poll_stored_info: OptionalFeatureVersion,
    pub fetch_identity_yes_no_vote: OptionalFeatureVersion,
    pub fetch_identities_voting_in_yes_no_vote_poll: OptionalFeatureVersion,
    pub remove_yes_no_vote_poll_votes_operations: OptionalFeatureVersion,
    pub remove_yes_no_vote_references_given_by_identity: OptionalFeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVoteFetchMethodVersions {
    pub fetch_identities_voting_for_contenders: FeatureVersion,
    pub fetch_contested_document_vote_poll_stored_info: FeatureVersion,
    pub fetch_identity_contested_resource_vote: FeatureVersion,
    /// Read by the contested document create state validation v2 (protocol version 14) only.
    pub fetch_contested_document_vote_poll_contender_count: FeatureVersion,
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
    /// Removes every vote a removed masternode cast. Version 1 (protocol version 15) also
    /// sweeps its yes/no votes.
    pub remove_all_votes_given_by_identities: FeatureVersion,
    /// Removes finished polls of every kind from the end date index. `None` (not active) before
    /// protocol version 15.
    pub remove_vote_poll_end_date_query_operations: OptionalFeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVoteInsertMethodVersions {
    pub register_identity_vote: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVoteContestedResourceInsertMethodVersions {
    pub register_contested_resource_identity_vote: FeatureVersion,
    pub insert_stored_info_for_contested_resource_vote_poll: FeatureVersion,
    /// Adds a poll to the end date index. Version 1 (protocol version 15) adds only the entry
    /// when the batch already queues the end date's tree with other storage flags.
    pub add_vote_poll_end_date_query_operations: FeatureVersion,
}
