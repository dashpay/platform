use versioned_feature_core::FeatureVersion;

pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;

#[derive(Clone, Debug, Default)]
pub struct DriveContractMethodVersions {
    pub prove: DriveContractProveMethodVersions,
    pub apply: DriveContractApplyMethodVersions,
    pub insert: DriveContractInsertMethodVersions,
    pub update: DriveContractUpdateMethodVersions,
    pub costs: DriveContractCostsMethodVersions,
    pub get: DriveContractGetMethodVersions,
    pub moderation: DriveContractModerationMethodVersions,
    pub fee_pots: DriveContractFeePotMethodVersions,
}

/// Drive methods for contract moderation: the banlist and the suspension list a moderated
/// contract keeps under its own subtree (keys `128` and `192` of its other tree, `[64, id, 2]`). Protocol version 14.
#[derive(Clone, Debug, Default)]
pub struct DriveContractModerationMethodVersions {
    pub add_contract_ban: FeatureVersion,
    pub remove_contract_ban: FeatureVersion,
    pub add_contract_suspension: FeatureVersion,
    pub remove_contract_suspension: FeatureVersion,
    pub add_contract_warning: FeatureVersion,
    pub remove_contract_warnings: FeatureVersion,
    pub fetch_contract_moderation_status: FeatureVersion,
    pub fetch_contract_moderation_entries: FeatureVersion,
    pub prove_contract_moderation_status: FeatureVersion,
    pub prove_contract_moderation_entries: FeatureVersion,
    pub insert_contract_moderation_trees: FeatureVersion,
    pub add_estimation_costs_for_contract_moderation_trees: FeatureVersion,
    pub add_estimation_costs_for_contract_moderation_entry: FeatureVersion,
    pub add_contract_document_removal: FeatureVersion,
    pub fetch_contract_document_removals: FeatureVersion,
    pub prove_contract_document_removals: FeatureVersion,
    pub insert_contract_document_removal_trees: FeatureVersion,
    pub add_estimation_costs_for_contract_document_removal: FeatureVersion,
    /// Writes a seated moderation team member's count of moderation actions since the last
    /// settle of the moderators pot (`[64, id, 2, 48]`, protocol version 14)
    pub set_contract_moderation_action_count: FeatureVersion,
    /// Reads every moderation action count of an elected contract
    pub fetch_contract_moderation_action_counts: FeatureVersion,
    /// Deletes moderation action counts: the reset at a settle of the moderators pot
    pub remove_contract_moderation_action_counts: FeatureVersion,
    pub add_estimation_costs_for_contract_moderation_action_counts: FeatureVersion,
}

/// Drive methods for the two fee pots a contract's document action fees accumulate in
/// (`[40, 64, contract id]` the owner pot, `[40, 192, contract id]` the moderators pot) and
/// the epoch each pot was last claimed in (keys `32` and `96` of the contract's other tree,
/// `[64, id, 2]`). Protocol version 14.
#[derive(Clone, Debug, Default)]
pub struct DriveContractFeePotMethodVersions {
    pub insert_contract_fee_pot_trees: FeatureVersion,
    pub add_to_contract_fee_pot: FeatureVersion,
    pub deduct_from_contract_fee_pot: FeatureVersion,
    pub fetch_contract_fee_pot: FeatureVersion,
    pub fetch_action_fee_multiplier: FeatureVersion,
    pub set_contract_last_fee_claim: FeatureVersion,
    pub prove_contract_fee_pots: FeatureVersion,
    pub add_estimation_costs_for_contract_fee_pot_update: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveContractProveMethodVersions {
    pub prove_contract: FeatureVersion,
    pub prove_contract_history: FeatureVersion,
    pub prove_contracts: FeatureVersion,
    pub prove_contracts_by_range: FeatureVersion,
    pub prove_contracts_versions: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveContractApplyMethodVersions {
    pub apply_contract: FeatureVersion,
    pub apply_contract_with_serialization: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveContractInsertMethodVersions {
    pub add_contract_to_storage: FeatureVersion,
    pub insert_contract: FeatureVersion,
    pub add_description: FeatureVersion,
    pub add_keywords: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveContractUpdateMethodVersions {
    pub update_contract: FeatureVersion,
    pub update_description: FeatureVersion,
    pub update_keywords: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveContractGetMethodVersions {
    pub fetch_contract: FeatureVersion,
    pub fetch_contract_ids: FeatureVersion,
    pub fetch_contract_version: FeatureVersion,
    pub fetch_contracts: FeatureVersion,
    pub fetch_contract_with_history: FeatureVersion,
    pub get_cached_contract_with_fetch_info: FeatureVersion,
    pub get_contract_with_fetch_info: FeatureVersion,
    pub get_contracts_with_fetch_info: FeatureVersion,
    pub get_system_or_user_contract_with_fee: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveContractQueryMethodVersions {
    pub fetch_contract_query: FeatureVersion,
    pub fetch_contract_with_history_latest_query: FeatureVersion,
    pub fetch_contracts_query: FeatureVersion,
    pub fetch_contract_history_query: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveContractCostsMethodVersions {
    pub add_estimation_costs_for_contract_insertion: FeatureVersion,
}
