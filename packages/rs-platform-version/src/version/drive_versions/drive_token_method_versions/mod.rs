use versioned_feature_core::{FeatureVersion, OptionalFeatureVersion};

pub mod v1;
pub mod v2;
pub mod v3;

#[derive(Clone, Debug, Default)]
pub struct DriveTokenMethodVersions {
    pub fetch: DriveTokenFetchMethodVersions,
    pub prove: DriveTokenProveMethodVersions,
    pub update: DriveTokenUpdateMethodVersions,
    pub calculate_total_tokens_balance: FeatureVersion,
    pub distribution: DriveTokenDistributionMethodVersions,
    /// The escrow of the token marketplace (protocol version 15): the tokens open listings hold
    /// and the credits open offers hold.
    pub order_escrow: DriveTokenOrderEscrowMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveTokenOrderEscrowMethodVersions {
    pub add_tokens: FeatureVersion,
    pub set_tokens: FeatureVersion,
    pub remove_tokens: FeatureVersion,
    pub fetch_tokens: FeatureVersion,
    pub add_credits: FeatureVersion,
    pub set_credits: FeatureVersion,
    pub remove_credits: FeatureVersion,
    pub fetch_credits: FeatureVersion,
    pub estimated_costs_for_tokens: FeatureVersion,
    pub estimated_costs_for_credits: FeatureVersion,
    /// Reads the open listings of a frozen identity that `TokenDestroyFrozenFunds` destroys with
    /// its balance. `None` reads none: the frozen balance alone is destroyed, as before the
    /// marketplace.
    pub fetch_frozen_listings: OptionalFeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveTokenDistributionMethodVersions {
    pub add_perpetual_distribution: FeatureVersion,
    pub add_pre_programmed_distributions: FeatureVersion,
    pub mark_perpetual_release_as_distributed: FeatureVersion,
    pub mark_pre_programmed_release_as_distributed: FeatureVersion,
    pub add_once_per_identity_distribution: FeatureVersion,
    pub mark_once_per_identity_release_as_distributed: FeatureVersion,
    /// The tokens an evonode earns from an `EvonodesByParticipation` perpetual distribution
    /// over one claim, and the moment the claim pays through.
    /// v0: reads the finalized epochs of the claim up to the query bound and evaluates the
    ///     whole claimed range, so a claim reaching past the epochs it read failed as an
    ///     internal error or, for a fixed amount, applied the share of the epochs read to the
    ///     whole range.
    /// v1: pays only through the last whole cycle it read, reading at least one whole cycle,
    ///     and counts an epoch without finalized info below the last one read as an epoch
    ///     without blocks.
    pub evonode_participation_rewards: FeatureVersion,
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
    pub once_per_identity_distribution_claim: FeatureVersion,
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
    /// Creates the per-token shielded pool subtree when a token with `has_shielded_pool` is
    /// registered.
    pub create_token_shielded_pool_trees: FeatureVersion,
    /// Identity token balance -> token shielded pool.
    pub shield: FeatureVersion,
    /// Token shielded pool -> identity token balance.
    pub unshield: FeatureVersion,
    /// Pool-internal token transfer.
    pub shielded_transfer: FeatureVersion,
    /// Mint straight into the token shielded pool (supply and pool balance both grow).
    pub mint_to_pool: FeatureVersion,
    /// Burn notes held in the token shielded pool (pool balance and supply both shrink).
    pub burn_from_pool: FeatureVersion,
}
