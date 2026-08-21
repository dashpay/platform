use grovedb_version::version::FeatureVersion;

pub mod v1;

#[derive(Clone, Debug, Default)]
pub struct DriveGroupMethodVersions {
    pub fetch: DriveGroupFetchMethodVersions,
    pub prove: DriveGroupProveMethodVersions,
    pub insert: DriveGroupInsertMethodVersions,
    pub cost_estimation: DriveGroupCostEstimationMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAddressFundsMethodVersions {
    pub set_balance_to_address: FeatureVersion,
    pub add_balance_to_address: FeatureVersion,
    pub remove_balance_from_address: FeatureVersion,
    pub fetch_balance_and_nonce: FeatureVersion,
    pub fetch_balances_with_nonces: FeatureVersion,
    pub prove_balance_and_nonce: FeatureVersion,
    pub prove_balances_with_nonces: FeatureVersion,
    pub prove_address_funds_trunk_query: FeatureVersion,
    pub prove_address_funds_branch_query: FeatureVersion,
    pub address_funds_query_min_depth: u8,
    pub address_funds_query_max_depth: u8,
    pub estimate_funding_fee: FeatureVersion,
    pub cost_estimation: DriveAddressFundsCostEstimationMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveGroupFetchMethodVersions {
    pub fetch_action_id_signers_power: FeatureVersion,
    pub fetch_active_action_info: FeatureVersion,
    pub fetch_action_id_info_keep_serialized: FeatureVersion,
    pub fetch_action_id_has_signer: FeatureVersion,
    pub fetch_group_info: FeatureVersion,
    pub fetch_group_infos: FeatureVersion,
    pub fetch_action_infos: FeatureVersion,
    pub fetch_action_signers: FeatureVersion,
    pub fetch_action_is_closed: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveGroupProveMethodVersions {
    pub prove_group_info: FeatureVersion,
    pub prove_group_infos: FeatureVersion,
    pub prove_action_infos: FeatureVersion,
    pub prove_action_signers: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveGroupInsertMethodVersions {
    pub add_new_groups: FeatureVersion,
    pub add_group_action: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveGroupCostEstimationMethodVersions {
    pub for_add_group_action: FeatureVersion,
    pub for_add_group: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAddressFundsCostEstimationMethodVersions {
    pub for_address_balance_update: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveShieldedMethodVersions {
    pub insert_note: FeatureVersion,
    pub insert_nullifiers: FeatureVersion,
    pub update_total_balance: FeatureVersion,
    pub record_anchor_if_changed: FeatureVersion,
    pub prune_anchors: FeatureVersion,
    pub has_anchor: FeatureVersion,
    pub has_nullifier: FeatureVersion,
    pub read_total_balance: FeatureVersion,
    pub notes_count: FeatureVersion,
}
