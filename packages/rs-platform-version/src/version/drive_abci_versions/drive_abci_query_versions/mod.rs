pub mod v1;

use versioned_feature_core::{FeatureVersion, FeatureVersionBounds};

#[derive(Clone, Debug, Default)]
pub struct DriveAbciQueryVersions {
    pub max_returned_elements: u16,
    pub response_metadata: FeatureVersion,
    pub proofs_query: FeatureVersion,
    pub document_query: FeatureVersionBounds,
    pub prefunded_specialized_balances: DriveAbciQueryPrefundedSpecializedBalancesVersions,
    pub identity_based_queries: DriveAbciQueryIdentityVersions,
    pub token_queries: DriveAbciQueryTokenVersions,
    pub validator_queries: DriveAbciQueryValidatorVersions,
    pub data_contract_based_queries: DriveAbciQueryDataContractVersions,
    pub voting_based_queries: DriveAbciQueryVotingVersions,
    pub system: DriveAbciQuerySystemVersions,
    pub group_queries: DriveAbciQueryGroupVersions,
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
pub struct DriveAbciQuerySystemVersions {
    pub version_upgrade_state: FeatureVersionBounds,
    pub version_upgrade_vote_status: FeatureVersionBounds,
    pub epoch_infos: FeatureVersionBounds,
    pub current_quorums_info: FeatureVersionBounds,
    pub partial_status: FeatureVersionBounds,
    pub path_elements: FeatureVersionBounds,
    pub total_credits_in_platform: FeatureVersionBounds,
}
