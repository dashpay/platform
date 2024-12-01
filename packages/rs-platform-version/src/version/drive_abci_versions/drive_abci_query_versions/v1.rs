use crate::version::drive_abci_versions::drive_abci_query_versions::{
    DriveAbciQueryDataContractVersions, DriveAbciQueryIdentityVersions,
    DriveAbciQueryPrefundedSpecializedBalancesVersions, DriveAbciQuerySystemVersions,
    DriveAbciQueryValidatorVersions, DriveAbciQueryVersions, DriveAbciQueryVotingVersions,
};
use versioned_feature_core::FeatureVersionBounds;

pub const DRIVE_ABCI_QUERY_VERSIONS_V1: DriveAbciQueryVersions = DriveAbciQueryVersions {
    max_returned_elements: 100,
    response_metadata: 0,
    proofs_query: FeatureVersionBounds {
        min_version: 0,
        max_version: 0,
        default_current_version: 0,
    },
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
        identities_contract_keys: FeatureVersionBounds {
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
        identity_by_public_key_hash: FeatureVersionBounds {
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
};
