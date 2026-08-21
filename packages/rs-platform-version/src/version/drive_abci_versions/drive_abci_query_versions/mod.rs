pub mod v0;
pub mod v1;
pub mod v2;
pub mod v3;

use versioned_feature_core::{FeatureVersion, FeatureVersionBounds};

#[derive(Clone, Debug, Default)]
pub struct DriveAbciQueryVersions {
    pub max_returned_elements: u16,
    pub response_metadata: FeatureVersion,
    pub proofs_query: FeatureVersion,
    pub document_query: FeatureVersionBounds,
    pub document_history: FeatureVersionBounds,
    /// Per-helper version slots for internal v1-document-query
    /// routing helpers. Separate from `document_query` (which
    /// versions the wire surface) because the helper output is
    /// consensus-relevant on the query path — adjusting the
    /// `(group_by × where)` routing table is a behavior change a
    /// future protocol version may need to make without re-cutting
    /// the wire shape.
    pub document_query_helpers: DriveAbciDocumentQueryHelperVersions,
    pub prefunded_specialized_balances: DriveAbciQueryPrefundedSpecializedBalancesVersions,
    pub identity_based_queries: DriveAbciQueryIdentityVersions,
    pub token_queries: DriveAbciQueryTokenVersions,
    pub validator_queries: DriveAbciQueryValidatorVersions,
    pub data_contract_based_queries: DriveAbciQueryDataContractVersions,
    pub voting_based_queries: DriveAbciQueryVotingVersions,
    pub system: DriveAbciQuerySystemVersions,
    pub group_queries: DriveAbciQueryGroupVersions,
    pub address_funds_queries: DriveAbciQueryAddressFundsVersions,
    pub shielded_queries: DriveAbciQueryShieldedVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciDocumentQueryHelperVersions {
    /// Version of the helper that picks the `(group_by × where)`
    /// mode for `SELECT COUNT / SUM / AVG` and enforces the
    /// per-mode `accepts_limit()` contract. See
    /// `query::document_query::v1::compute_aggregate_mode_and_check_limit`.
    pub compute_aggregate_mode_and_check_limit: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciQueryPrefundedSpecializedBalancesVersions {
    pub balance: FeatureVersionBounds,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciQueryTokenVersions {
    pub identity_token_balances: FeatureVersionBounds,
    pub identities_token_balances: FeatureVersionBounds,
    pub identities_token_infos: FeatureVersionBounds,
    pub identity_token_infos: FeatureVersionBounds,
    pub token_statuses: FeatureVersionBounds,
    pub token_total_supply: FeatureVersionBounds,
    pub token_direct_purchase_prices: FeatureVersionBounds,
    pub token_pre_programmed_distributions: FeatureVersionBounds,
    pub token_perpetual_distribution_last_claim: FeatureVersionBounds,
    pub token_contract_info: FeatureVersionBounds,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciQueryGroupVersions {
    pub group_info: FeatureVersionBounds,
    pub group_infos: FeatureVersionBounds,
    pub group_actions: FeatureVersionBounds,
    pub group_action_signers: FeatureVersionBounds,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciQueryAddressFundsVersions {
    pub addresses_infos: FeatureVersionBounds,
    pub address_info: FeatureVersionBounds,
    pub addresses_trunk_state: FeatureVersionBounds,
    pub addresses_branch_state: FeatureVersionBounds,
    pub recent_address_balance_changes: FeatureVersionBounds,
    pub recent_compacted_address_balance_changes: FeatureVersionBounds,
    pub address_funding_fee_quote: FeatureVersionBounds,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciQueryIdentityVersions {
    pub identity: FeatureVersionBounds,
    pub identities_contract_keys: FeatureVersionBounds,
    pub keys: FeatureVersionBounds,
    pub identity_nonce: FeatureVersionBounds,
    pub identity_contract_nonce: FeatureVersionBounds,
    pub balance: FeatureVersionBounds,
    pub identities_balances: FeatureVersionBounds,
    pub balance_and_revision: FeatureVersionBounds,
    pub identity_by_unique_public_key_hash: FeatureVersionBounds,
    pub identity_by_non_unique_public_key_hash: FeatureVersionBounds,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciQueryValidatorVersions {
    pub proposed_block_counts_by_evonode_ids: FeatureVersionBounds,
    pub proposed_block_counts_by_range: FeatureVersionBounds,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciQueryVotingVersions {
    pub vote_polls_by_end_date_query: FeatureVersionBounds,
    pub contested_resource_vote_state: FeatureVersionBounds,
    pub contested_resource_voters_for_identity: FeatureVersionBounds,
    pub contested_resource_identity_vote_status: FeatureVersionBounds,
    pub contested_resources: FeatureVersionBounds,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciQueryDataContractVersions {
    pub data_contract: FeatureVersionBounds,
    pub data_contract_history: FeatureVersionBounds,
    pub data_contracts: FeatureVersionBounds,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciQueryShieldedVersions {
    pub encrypted_notes: FeatureVersionBounds,
    pub anchors: FeatureVersionBounds,
    pub most_recent_anchor: FeatureVersionBounds,
    pub pool_state: FeatureVersionBounds,
    pub notes_count: FeatureVersionBounds,
    pub nullifiers: FeatureVersionBounds,
    /// Maximum number of MMR chunks a single `getShieldedEncryptedNotes`
    /// query may span.
    ///
    /// The wire-level cap on notes returned per query is therefore
    /// `max_query_chunks × (1 << SHIELDED_NOTES_CHUNK_POWER)` — today
    /// `chunk_power = 11` so each chunk holds 2048 notes. `start_index`
    /// must still be chunk-aligned (2048-note boundary); this cap only
    /// controls how many adjacent chunks one proof may cover.
    ///
    /// Expressed in chunks (not raw notes) so the MMR-shape coupling
    /// is explicit and the cap can be bumped independently of the
    /// chunk size. v0 = 1 (legacy single-chunk-per-query behaviour);
    /// v1 = 4 (8192-note responses, ~4× fewer round-trips on a cold
    /// 1M-note sync).
    pub max_query_chunks: u8,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciQuerySystemVersions {
    pub version_upgrade_state: FeatureVersionBounds,
    pub version_upgrade_vote_status: FeatureVersionBounds,
    pub epoch_infos: FeatureVersionBounds,
    pub current_quorums_info: FeatureVersionBounds,
    pub partial_status: FeatureVersionBounds,
    pub path_elements: FeatureVersionBounds,
    pub total_credits_in_platform: FeatureVersionBounds,
    pub finalized_epoch_infos: FeatureVersionBounds,
}
