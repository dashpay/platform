use versioned_feature_core::FeatureVersion;

pub mod v1;

#[derive(Clone, Debug, Default)]
pub struct DriveTokenMethodVersions {
    pub fetch: DriveTokenFetchMethodVersions,
    pub prove: DriveTokenProveMethodVersions,
    pub update: DriveTokenUpdateMethodVersions,
    pub calculate_total_tokens_balance: FeatureVersion,
    pub distribution: DriveTokenDistributionMethodVersions,
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
}
