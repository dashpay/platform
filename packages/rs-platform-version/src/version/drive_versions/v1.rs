use crate::version::drive_versions::drive_contract_method_versions::v1::DRIVE_CONTRACT_METHOD_VERSIONS_V1;
use crate::version::drive_versions::drive_credit_pool_method_versions::v1::CREDIT_POOL_METHOD_VERSIONS_V1;
use crate::version::drive_versions::drive_document_method_versions::v1::DRIVE_DOCUMENT_METHOD_VERSIONS_V1;
use crate::version::drive_versions::drive_group_method_versions::v1::DRIVE_GROUP_METHOD_VERSIONS_V1;
use crate::version::drive_versions::drive_grove_method_versions::v1::DRIVE_GROVE_METHOD_VERSIONS_V1;
use crate::version::drive_versions::drive_identity_method_versions::v1::DRIVE_IDENTITY_METHOD_VERSIONS_V1;
use crate::version::drive_versions::drive_state_transition_method_versions::v1::DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V1;
use crate::version::drive_versions::drive_structure_version::v1::DRIVE_STRUCTURE_V1;
use crate::version::drive_versions::drive_token_method_versions::v1::DRIVE_TOKEN_METHOD_VERSIONS_V1;
use crate::version::drive_versions::drive_verify_method_versions::v1::DRIVE_VERIFY_METHOD_VERSIONS_V1;
use crate::version::drive_versions::drive_vote_method_versions::v1::DRIVE_VOTE_METHOD_VERSIONS_V1;
use crate::version::drive_versions::{
    DriveAssetLockMethodVersions, DriveBalancesMethodVersions, DriveBatchOperationsMethodVersion,
    DriveEstimatedCostsMethodVersions, DriveFeesMethodVersions, DriveFetchMethodVersions,
    DriveInitializationMethodVersions, DriveMethodVersions, DriveOperationsMethodVersion,
    DrivePlatformStateMethodVersions, DrivePlatformSystemMethodVersions,
    DrivePrefundedSpecializedMethodVersions, DriveProtocolUpgradeVersions,
    DriveProveMethodVersions, DriveSystemEstimationCostsMethodVersions, DriveVersion,
};
use grovedb_version::version::v1::GROVE_V1;

pub const DRIVE_VERSION_V1: DriveVersion = DriveVersion {
    structure: DRIVE_STRUCTURE_V1,
    methods: DriveMethodVersions {
        initialization: DriveInitializationMethodVersions {
            create_initial_state_structure: 0,
        },
        credit_pools: CREDIT_POOL_METHOD_VERSIONS_V1,
        protocol_upgrade: DriveProtocolUpgradeVersions {
            clear_version_information: 0,
            fetch_versions_with_counter: 0,
            fetch_proved_versions_with_counter: 0,
            fetch_validator_version_votes: 0,
            fetch_proved_validator_version_votes: 0,
            remove_validators_proposed_app_versions: 0,
            update_validator_proposed_app_version: 0,
        },
        prove: DriveProveMethodVersions {
            prove_elements: 0,
            prove_multiple_state_transition_results: 0,
            prove_state_transition: 0,
        },
        balances: DriveBalancesMethodVersions {
            add_to_system_credits: 0,
            add_to_system_credits_operations: 0,
            remove_from_system_credits: 0,
            remove_from_system_credits_operations: 0,
            calculate_total_credits_balance: 0,
        },
        document: DRIVE_DOCUMENT_METHOD_VERSIONS_V1,
        vote: DRIVE_VOTE_METHOD_VERSIONS_V1,
        contract: DRIVE_CONTRACT_METHOD_VERSIONS_V1,
        fees: DriveFeesMethodVersions { calculate_fee: 0 },
        estimated_costs: DriveEstimatedCostsMethodVersions {
            add_estimation_costs_for_levels_up_to_contract: 0,
            add_estimation_costs_for_levels_up_to_contract_document_type_excluded: 0,
            add_estimation_costs_for_contested_document_tree_levels_up_to_contract: 0,
            add_estimation_costs_for_contested_document_tree_levels_up_to_contract_document_type_excluded: 0,
        },
        asset_lock: DriveAssetLockMethodVersions {
            add_asset_lock_outpoint: 0,
            add_estimation_costs_for_adding_asset_lock: 0,
            fetch_asset_lock_outpoint_info: 0,
        },
        verify: DRIVE_VERIFY_METHOD_VERSIONS_V1,
        identity: DRIVE_IDENTITY_METHOD_VERSIONS_V1,
        token: DRIVE_TOKEN_METHOD_VERSIONS_V1,
        platform_system: DrivePlatformSystemMethodVersions {
            estimation_costs: DriveSystemEstimationCostsMethodVersions {
                for_total_system_credits_update: 0,
            },
        },
        operations: DriveOperationsMethodVersion {
            rollback_transaction: 0,
            drop_cache: 0,
            commit_transaction: 0,
            apply_partial_batch_low_level_drive_operations: 0,
            apply_partial_batch_grovedb_operations: 0,
            apply_batch_low_level_drive_operations: 0,
            apply_batch_grovedb_operations: 0,
        },
        state_transitions: DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V1,
        batch_operations: DriveBatchOperationsMethodVersion {
            convert_drive_operations_to_grove_operations: 0,
            apply_drive_operations: 0,
        },
        platform_state: DrivePlatformStateMethodVersions {
            fetch_platform_state_bytes: 0,
            store_platform_state_bytes: 0,
        },
        fetch: DriveFetchMethodVersions { fetch_elements: 0 },
        prefunded_specialized_balances: DrivePrefundedSpecializedMethodVersions {
            fetch_single: 0,
            prove_single: 0,
            add_prefunded_specialized_balance: 0,
            add_prefunded_specialized_balance_operations: 0,
            deduct_from_prefunded_specialized_balance: 0,
            deduct_from_prefunded_specialized_balance_operations: 0,
            estimated_cost_for_prefunded_specialized_balance_update: 0,
            empty_prefunded_specialized_balance: 0,
        },
        group: DRIVE_GROUP_METHOD_VERSIONS_V1,
    },
    grove_methods: DRIVE_GROVE_METHOD_VERSIONS_V1,
    grove_version: GROVE_V1,
};
