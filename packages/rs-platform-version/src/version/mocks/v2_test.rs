use crate::version::consensus_versions::ConsensusVersions;
use crate::version::dpp_versions::dpp_asset_lock_versions::v1::DPP_ASSET_LOCK_VERSIONS_V1;
use crate::version::dpp_versions::dpp_contract_versions::v1::CONTRACT_VERSIONS_V1;
use crate::version::dpp_versions::dpp_costs_versions::v1::DPP_COSTS_VERSIONS_V1;
use crate::version::dpp_versions::dpp_document_versions::v1::DOCUMENT_VERSIONS_V1;
use crate::version::dpp_versions::dpp_factory_versions::v1::DPP_FACTORY_VERSIONS_V1;
use crate::version::dpp_versions::dpp_identity_versions::v1::IDENTITY_VERSIONS_V1;
use crate::version::dpp_versions::dpp_method_versions::v1::DPP_METHOD_VERSIONS_V1;
use crate::version::dpp_versions::dpp_state_transition_conversion_versions::v1::STATE_TRANSITION_CONVERSION_VERSIONS_V1;
use crate::version::dpp_versions::dpp_state_transition_method_versions::v1::STATE_TRANSITION_METHOD_VERSIONS_V1;
use crate::version::dpp_versions::dpp_state_transition_serialization_versions::v1::STATE_TRANSITION_SERIALIZATION_VERSIONS_V1;
use crate::version::dpp_versions::dpp_state_transition_versions::v1::STATE_TRANSITION_VERSIONS_V1;
use crate::version::dpp_versions::dpp_token_versions::v1::TOKEN_VERSIONS_V1;
use crate::version::dpp_versions::dpp_validation_versions::v2::DPP_VALIDATION_VERSIONS_V2;
use crate::version::dpp_versions::dpp_voting_versions::v2::VOTING_VERSION_V2;
use crate::version::dpp_versions::DPPVersion;
use crate::version::drive_abci_versions::drive_abci_method_versions::v1::DRIVE_ABCI_METHOD_VERSIONS_V1;
use crate::version::drive_abci_versions::drive_abci_query_versions::{
    DriveAbciQueryDataContractVersions, DriveAbciQueryGroupVersions,
    DriveAbciQueryIdentityVersions, DriveAbciQueryPrefundedSpecializedBalancesVersions,
    DriveAbciQuerySystemVersions, DriveAbciQueryTokenVersions, DriveAbciQueryValidatorVersions,
    DriveAbciQueryVersions, DriveAbciQueryVotingVersions,
};
use crate::version::drive_abci_versions::drive_abci_structure_versions::v1::DRIVE_ABCI_STRUCTURE_VERSIONS_V1;
use crate::version::drive_abci_versions::drive_abci_validation_versions::v1::DRIVE_ABCI_VALIDATION_VERSIONS_V1;
use crate::version::drive_abci_versions::drive_abci_withdrawal_constants::v1::DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V1;
use crate::version::drive_abci_versions::DriveAbciVersion;
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
use crate::version::fee::v1::FEE_VERSION1;
use crate::version::mocks::TEST_PROTOCOL_VERSION_SHIFT_BYTES;
use crate::version::protocol_version::{FeatureVersionBounds, PlatformVersion};
use crate::version::system_data_contract_versions::v1::SYSTEM_DATA_CONTRACT_VERSIONS_V1;
use crate::version::system_limits::SystemLimits;
use grovedb_version::version::v1::GROVE_V1;

pub const TEST_PROTOCOL_VERSION_2: u32 = (1 << TEST_PROTOCOL_VERSION_SHIFT_BYTES) + 2;

pub const TEST_PLATFORM_V2: PlatformVersion = PlatformVersion {
    protocol_version: TEST_PROTOCOL_VERSION_2,
    drive: DriveVersion {
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
            batch_operations: DriveBatchOperationsMethodVersion {
                convert_drive_operations_to_grove_operations: 0,
                apply_drive_operations: 0,
            },
            state_transitions: DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V1,
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
    },
    drive_abci: DriveAbciVersion {
        structs: DRIVE_ABCI_STRUCTURE_VERSIONS_V1,
        methods: DRIVE_ABCI_METHOD_VERSIONS_V1,
        validation_and_processing: DRIVE_ABCI_VALIDATION_VERSIONS_V1,
        withdrawal_constants: DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V1,
        query: DriveAbciQueryVersions {
            max_returned_elements: 100,
            response_metadata: 0,
            proofs_query: 0,
            document_query: FeatureVersionBounds {
                min_version: 0,
                max_version: 0,
                default_current_version: 0,
            },
            prefunded_specialized_balances: DriveAbciQueryPrefundedSpecializedBalancesVersions {
                balance: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
            },
            identity_based_queries: DriveAbciQueryIdentityVersions {
                identity: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                keys: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                identity_nonce: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                identity_contract_nonce: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                balance: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                identities_balances: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                balance_and_revision: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                identity_by_unique_public_key_hash: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                identities_contract_keys: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                identity_by_non_unique_public_key_hash: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
            },
            token_queries: DriveAbciQueryTokenVersions {
                identity_token_balances: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                identities_token_balances: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                identities_token_infos: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                identity_token_infos: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                token_statuses: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                token_total_supply: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                token_direct_purchase_prices:FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                token_pre_programmed_distributions: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                token_perpetual_distribution_last_claim: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                token_contract_info: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
            },
            validator_queries: DriveAbciQueryValidatorVersions {
                proposed_block_counts_by_evonode_ids: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                proposed_block_counts_by_range: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
            },
            data_contract_based_queries: DriveAbciQueryDataContractVersions {
                data_contract: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                data_contract_history: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                data_contracts: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
            },
            voting_based_queries: DriveAbciQueryVotingVersions {
                vote_polls_by_end_date_query: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                contested_resource_vote_state: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                contested_resource_voters_for_identity: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                contested_resource_identity_vote_status: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                contested_resources: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
            },
            system: DriveAbciQuerySystemVersions {
                version_upgrade_state: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                version_upgrade_vote_status: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                epoch_infos: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                current_quorums_info: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                partial_status: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                path_elements: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                total_credits_in_platform: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
            },
            group_queries: DriveAbciQueryGroupVersions {
                group_info: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                group_infos: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                group_actions: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
                group_action_signers: FeatureVersionBounds {
                    min_version: 0,
                    max_version: 0,
                    default_current_version: 0,
                },
            },
        },
    },
    dpp: DPPVersion {
        costs: DPP_COSTS_VERSIONS_V1,
        validation: DPP_VALIDATION_VERSIONS_V2,
        state_transition_serialization_versions: STATE_TRANSITION_SERIALIZATION_VERSIONS_V1,
        state_transition_conversion_versions: STATE_TRANSITION_CONVERSION_VERSIONS_V1,
        state_transition_method_versions: STATE_TRANSITION_METHOD_VERSIONS_V1,
        state_transitions: STATE_TRANSITION_VERSIONS_V1,
        contract_versions: CONTRACT_VERSIONS_V1,
        document_versions: DOCUMENT_VERSIONS_V1,
        identity_versions: IDENTITY_VERSIONS_V1,
        voting_versions: VOTING_VERSION_V2,
        token_versions: TOKEN_VERSIONS_V1,
        asset_lock_versions: DPP_ASSET_LOCK_VERSIONS_V1,
        methods: DPP_METHOD_VERSIONS_V1,
        factory_versions: DPP_FACTORY_VERSIONS_V1,
    },
    system_data_contracts: SYSTEM_DATA_CONTRACT_VERSIONS_V1,
    fee_version: FEE_VERSION1,
    system_limits: SystemLimits {
        estimated_contract_max_serialized_size: 16384,
        max_field_value_size: 5000,
        max_state_transition_size: 20000, // Is different in this test version, not sure if this was a mistake
        max_transitions_in_documents_batch: 1,
        withdrawal_transactions_per_block_limit: 4,
        retry_signing_expired_withdrawal_documents_per_block_limit: 1,
        max_withdrawal_amount: 50_000_000_000_000,
        max_contract_group_size: 256,
        max_token_redemption_cycles: 128,
    },
    consensus: ConsensusVersions {
        tenderdash_consensus_version: 0,
    },
};
