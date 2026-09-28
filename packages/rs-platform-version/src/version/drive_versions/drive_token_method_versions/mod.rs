use versioned_feature_core::{FeatureVersion, OptionalFeatureVersion};

pub mod v1;
pub mod v2;

#[derive(Clone, Debug, Default)]
pub struct DriveTokenMethodVersions {
    pub fetch: DriveTokenFetchMethodVersions,
    pub prove: DriveTokenProveMethodVersions,
    pub update: DriveTokenUpdateMethodVersions,
    pub calculate_total_tokens_balance: FeatureVersion,
    pub distribution: DriveTokenDistributionMethodVersions,
    pub lifecycle: DriveTokenLifecycleMethodVersions,
}

/// Per-issuer token lifecycle: the supply rollup every contract that issues
/// tokens carries, the destroyed-issuer ledger and the reads that resolve a
/// token to its issuer's state. Every slot is optional because the ledger
/// (`[Tokens] / 224`) only exists from the protocol version that introduces
/// it; a shipped table keeps `None` and the dispatchers report
/// `VersionNotActive`.
#[derive(Clone, Debug, Default)]
pub struct DriveTokenLifecycleMethodVersions {
    /// Reads one contract's lifecycle record (supply rollup and wipe marker).
    pub fetch_contract_token_lifecycle: OptionalFeatureVersion,
    /// Resolves token ids to their issuer's lifecycle through the contract
    /// info leaves.
    pub fetch_token_lifecycles: OptionalFeatureVersion,
    /// Moves a token's issuer rollup by the delta a supply write applies,
    /// refusing wiped issuers. Called by `add_to_token_total_supply` and
    /// `remove_from_token_total_supply` from their generation 1 on.
    pub add_to_contract_issued_supply: OptionalFeatureVersion,
    /// Marks an issuer wiped and moves its rollup into the destroyed supply
    /// ledger in one bounded batch.
    pub destroy_token_issuer: OptionalFeatureVersion,
    /// Layer estimation for writes under the lifecycle ledger.
    pub add_estimation_costs_for_token_contract_lifecycles: OptionalFeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveTokenDistributionMethodVersions {
    pub add_perpetual_distribution: FeatureVersion,
    pub add_pre_programmed_distributions: FeatureVersion,
    pub mark_perpetual_release_as_distributed: FeatureVersion,
    pub mark_pre_programmed_release_as_distributed: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveTokenFetchMethodVersions {
    pub identity_token_balance: FeatureVersion,
    pub identity_token_balances: FeatureVersion,
    pub identities_token_balances: FeatureVersion,
    pub identity_token_info: FeatureVersion,
    pub identity_token_infos: FeatureVersion,
    pub identities_token_infos: FeatureVersion,
    pub token_statuses: FeatureVersion,
    pub token_status: FeatureVersion,
    pub token_total_supply: FeatureVersion,
    pub token_total_aggregated_identity_balances: FeatureVersion,
    pub pre_programmed_distributions: FeatureVersion,
    pub perpetual_distribution_last_paid_time: FeatureVersion,
    pub pre_programmed_distribution_last_paid_time: FeatureVersion,
    pub token_direct_purchase_price: FeatureVersion,
    pub token_direct_purchase_prices: FeatureVersion,
    pub token_contract_info: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveTokenProveMethodVersions {
    pub identity_token_balance: FeatureVersion,
    pub identity_token_balances: FeatureVersion,
    pub identities_token_balances: FeatureVersion,
    pub identity_token_info: FeatureVersion,
    pub identity_token_infos: FeatureVersion,
    pub identities_token_infos: FeatureVersion,
    pub token_statuses: FeatureVersion,
    pub total_supply_and_aggregated_identity_balances: FeatureVersion,
    pub pre_programmed_distributions: FeatureVersion,
    pub token_direct_purchase_prices: FeatureVersion,
    pub perpetual_distribution_last_paid_time: FeatureVersion,
    pub token_contract_info: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveTokenUpdateMethodVersions {
    pub create_token_trees: FeatureVersion,
    pub burn: FeatureVersion,
    pub mint: FeatureVersion,
    pub mint_many: FeatureVersion,
    pub transfer: FeatureVersion,
    pub add_to_token_total_supply: FeatureVersion,
    pub remove_from_token_total_supply: FeatureVersion,
    pub remove_from_identity_token_balance: FeatureVersion,
    pub add_to_identity_token_balance: FeatureVersion,
    pub add_transaction_history_operations: FeatureVersion,
    pub freeze: FeatureVersion,
    pub unfreeze: FeatureVersion,
    pub apply_status: FeatureVersion,
    pub perpetual_distribution_next_event_for_identity_id: FeatureVersion,
    /// Generation of `token_set_direct_purchase_price_operations`. The dispatcher read the
    /// `mint` slot before this field existed; the shipped tables carry 0 here, which is what
    /// `mint` was on every one of them, so the selection is unchanged.
    pub set_direct_purchase_price: FeatureVersion,
}
