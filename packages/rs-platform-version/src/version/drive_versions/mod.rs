use crate::version::FeatureVersion;
use drive_contract_method_versions::DriveContractMethodVersions;
use drive_credit_pool_method_versions::DriveCreditPoolMethodVersions;
use drive_document_method_versions::DriveDocumentMethodVersions;
use drive_group_method_versions::DriveGroupMethodVersions;
use drive_grove_method_versions::DriveGroveMethodVersions;
use drive_identity_method_versions::DriveIdentityMethodVersions;
use drive_state_transition_method_versions::DriveStateTransitionMethodVersions;
use drive_structure_version::DriveStructureVersion;
use drive_token_method_versions::DriveTokenMethodVersions;
use drive_verify_method_versions::DriveVerifyMethodVersions;
use drive_vote_method_versions::DriveVoteMethodVersions;
use grovedb_version::version::GroveVersion;

pub mod drive_contract_method_versions;
pub mod drive_credit_pool_method_versions;
pub mod drive_document_method_versions;
pub mod drive_group_method_versions;
pub mod drive_grove_method_versions;
pub mod drive_identity_method_versions;
pub mod drive_state_transition_method_versions;
pub mod drive_structure_version;
pub mod drive_token_method_versions;
pub mod drive_verify_method_versions;
pub mod drive_vote_method_versions;
pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;

#[derive(Clone, Debug, Default)]
pub struct DriveVersion {
    pub structure: DriveStructureVersion,
    pub methods: DriveMethodVersions,
    pub grove_methods: DriveGroveMethodVersions,
    pub grove_version: GroveVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveMethodVersions {
    pub initialization: DriveInitializationMethodVersions,
    pub credit_pools: DriveCreditPoolMethodVersions,
    pub protocol_upgrade: DriveProtocolUpgradeVersions,
    pub prefunded_specialized_balances: DrivePrefundedSpecializedMethodVersions,
    pub balances: DriveBalancesMethodVersions,
    pub document: DriveDocumentMethodVersions,
    pub vote: DriveVoteMethodVersions,
    pub contract: DriveContractMethodVersions,
    pub fees: DriveFeesMethodVersions,
    pub estimated_costs: DriveEstimatedCostsMethodVersions,
    pub asset_lock: DriveAssetLockMethodVersions,
    pub verify: DriveVerifyMethodVersions,
    pub identity: DriveIdentityMethodVersions,
    pub token: DriveTokenMethodVersions,
    pub platform_system: DrivePlatformSystemMethodVersions,
    pub operations: DriveOperationsMethodVersion,
    pub batch_operations: DriveBatchOperationsMethodVersion,
    pub fetch: DriveFetchMethodVersions,
    pub prove: DriveProveMethodVersions,
    pub state_transitions: DriveStateTransitionMethodVersions,
    pub platform_state: DrivePlatformStateMethodVersions,
    pub group: DriveGroupMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DrivePlatformStateMethodVersions {
    pub fetch_platform_state_bytes: FeatureVersion,
    pub store_platform_state_bytes: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveDataContractOperationMethodVersions {
    pub finalization_tasks: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveFetchMethodVersions {
    pub fetch_elements: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveProveMethodVersions {
    pub prove_elements: FeatureVersion,
    pub prove_multiple_state_transition_results: FeatureVersion,
    pub prove_state_transition: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DrivePrefundedSpecializedMethodVersions {
    pub fetch_single: FeatureVersion,
    pub prove_single: FeatureVersion,
    pub add_prefunded_specialized_balance: FeatureVersion,
    pub add_prefunded_specialized_balance_operations: FeatureVersion,
    pub deduct_from_prefunded_specialized_balance: FeatureVersion,
    pub deduct_from_prefunded_specialized_balance_operations: FeatureVersion,
    pub estimated_cost_for_prefunded_specialized_balance_update: FeatureVersion,
    pub empty_prefunded_specialized_balance: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveBalancesMethodVersions {
    pub add_to_system_credits: FeatureVersion,
    pub add_to_system_credits_operations: FeatureVersion,
    pub remove_from_system_credits: FeatureVersion,
    pub remove_from_system_credits_operations: FeatureVersion,
    pub calculate_total_credits_balance: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAssetLockMethodVersions {
    pub add_asset_lock_outpoint: FeatureVersion,
    pub add_estimation_costs_for_adding_asset_lock: FeatureVersion,
    pub fetch_asset_lock_outpoint_info: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveFeesMethodVersions {
    pub calculate_fee: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DrivePlatformSystemMethodVersions {
    pub estimation_costs: DriveSystemEstimationCostsMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveOperationsMethodVersion {
    pub rollback_transaction: FeatureVersion,
    pub drop_cache: FeatureVersion,
    pub commit_transaction: FeatureVersion,
    pub apply_partial_batch_low_level_drive_operations: FeatureVersion,
    pub apply_partial_batch_grovedb_operations: FeatureVersion,
    pub apply_batch_low_level_drive_operations: FeatureVersion,
    pub apply_batch_grovedb_operations: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveBatchOperationsMethodVersion {
    pub convert_drive_operations_to_grove_operations: FeatureVersion,
    pub apply_drive_operations: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveSystemEstimationCostsMethodVersions {
    pub for_total_system_credits_update: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveInitializationMethodVersions {
    pub create_initial_state_structure: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveProtocolUpgradeVersions {
    pub clear_version_information: FeatureVersion,
    pub fetch_versions_with_counter: FeatureVersion,
    pub fetch_proved_versions_with_counter: FeatureVersion,
    pub fetch_validator_version_votes: FeatureVersion,
    pub fetch_proved_validator_version_votes: FeatureVersion,
    pub remove_validators_proposed_app_versions: FeatureVersion,
    pub update_validator_proposed_app_version: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveEstimatedCostsMethodVersions {
    pub add_estimation_costs_for_levels_up_to_contract: FeatureVersion,
    pub add_estimation_costs_for_levels_up_to_contract_document_type_excluded: FeatureVersion,
    pub add_estimation_costs_for_contested_document_tree_levels_up_to_contract: FeatureVersion,
    pub add_estimation_costs_for_contested_document_tree_levels_up_to_contract_document_type_excluded:
        FeatureVersion,
}
