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
    pub readiness: DriveVoteReadinessMethodVersions,
}

/// Compilation readiness rounds, reports, cursors, deadlines and funds: the state Drive keeps
/// under `[Votes] / r` and `[PreFundedSpecializedBalances] / 129` from the protocol version
/// that introduces them. Every slot is optional because the subtrees only exist from that
/// version; a shipped table keeps `None` and the dispatchers report `VersionNotActive`.
#[derive(Clone, Debug, Default)]
pub struct DriveVoteReadinessMethodVersions {
    /// Opens a contract's round, retiring the previous one and funding the new one.
    pub open_round: OptionalFeatureVersion,
    /// Cancels a contract's round, retiring it and refunding its fund.
    pub cancel_round: OptionalFeatureVersion,
    /// Retires a round already unlinked from a contract into the cleanup queue and settles
    /// its fund; the primitive `open_round`, `cancel_round` and `activate_round` share.
    pub retire_round: OptionalFeatureVersion,
    /// Inserts one report into the current round's count tree if absent.
    pub insert_report: OptionalFeatureVersion,
    /// Reads a contract's current round record through the pointer.
    pub fetch_round: OptionalFeatureVersion,
    /// Reads the raw distinct report count of a round.
    pub fetch_raw_count: OptionalFeatureVersion,
    /// Reads a page of a round's reports in key order.
    pub fetch_reports_page: OptionalFeatureVersion,
    /// Reads a page of contracts holding a round, in contract id order.
    pub fetch_rounds_page: OptionalFeatureVersion,
    /// Writes the persisted position of a paged eligibility walk.
    pub store_scan_cursor: OptionalFeatureVersion,
    /// Reads the persisted position of a paged eligibility walk.
    pub fetch_scan_cursor: OptionalFeatureVersion,
    /// Removes the persisted position of a paged eligibility walk.
    pub clear_scan_cursor: OptionalFeatureVersion,
    /// Rewrites a round record with its last evaluation mark.
    pub update_round_evaluation: OptionalFeatureVersion,
    /// Marks a round crossed and queues its activation deadline.
    pub record_crossing: OptionalFeatureVersion,
    /// Reads the rounds whose activation deadline has passed.
    pub fetch_rounds_due: OptionalFeatureVersion,
    /// Activates a round at its deadline: unlinks it, retires it, settles the fund.
    pub activate_round: OptionalFeatureVersion,
    /// Deletes named reports from a round's count tree.
    pub prune_reports: OptionalFeatureVersion,
    /// Reads the first retired round awaiting cleanup.
    pub fetch_retired_round: OptionalFeatureVersion,
    /// Runs one bounded cleanup step over a retired round.
    pub cleanup_retired_round: OptionalFeatureVersion,
    /// Writes the block event's fairness cursor over contracts.
    pub store_evaluation_cursor: OptionalFeatureVersion,
    /// Reads the block event's fairness cursor over contracts.
    pub fetch_evaluation_cursor: OptionalFeatureVersion,
    /// Creates or tops up a readiness fund under `[PreFundedSpecializedBalances] / 129`.
    pub add_fund: OptionalFeatureVersion,
    /// Deducts from a readiness fund.
    pub deduct_from_fund: OptionalFeatureVersion,
    /// Empties a readiness fund, returning what it held.
    pub empty_fund: OptionalFeatureVersion,
    /// Reads a readiness fund.
    pub fetch_fund: OptionalFeatureVersion,
    /// Proves a readiness fund.
    pub prove_fund: OptionalFeatureVersion,
    /// Layer estimation for writes under the readiness subtrees.
    pub estimation_costs: OptionalFeatureVersion,
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
