use crate::version::drive_abci_versions::drive_abci_query_versions::{
    DriveAbciDocumentQueryHelperVersions, DriveAbciQueryAddressFundsVersions,
    DriveAbciQueryDataContractVersions, DriveAbciQueryGroupVersions,
    DriveAbciQueryIdentityVersions, DriveAbciQueryPrefundedSpecializedBalancesVersions,
    DriveAbciQueryShieldedVersions, DriveAbciQuerySystemVersions, DriveAbciQueryTokenVersions,
    DriveAbciQueryValidatorVersions, DriveAbciQueryVersions, DriveAbciQueryVotingVersions,
};
use versioned_feature_core::FeatureVersionBounds;

/// `DRIVE_ABCI_QUERY_VERSIONS_V0` — query feature-version state before
/// `getDocuments` advanced to V1 (#3633). All fields IDENTICAL to V1 except
/// `document_query`, which pins both `max_version` and `default_current_version`
/// to 0 so clients pinned to a PV using this module emit V0 wire bytes (CBOR
/// `where` / `order_by`, plain `uint32 limit`). This is the version testnet
/// v3.0 HPMNs deserialize.
pub const DRIVE_ABCI_QUERY_VERSIONS_V0: DriveAbciQueryVersions = DriveAbciQueryVersions {
    max_returned_elements: 100,
    response_metadata: 0,
    proofs_query: 0,
    document_query: FeatureVersionBounds {
        min_version: 0,
        max_version: 0,
        default_current_version: 0,
    },
    document_history: FeatureVersionBounds {
        min_version: 0,
        max_version: 0,
        default_current_version: 0,
    },
    document_query_helpers: DriveAbciDocumentQueryHelperVersions {
        compute_aggregate_mode_and_check_limit: 0,
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
        identity_by_unique_public_key_hash: FeatureVersionBounds {
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
        token_direct_purchase_prices: FeatureVersionBounds {
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
        finalized_epoch_infos: FeatureVersionBounds {
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
    shielded_queries: DriveAbciQueryShieldedVersions {
        encrypted_notes: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        anchors: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        most_recent_anchor: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        pool_state: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        notes_count: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        nullifiers: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        max_query_chunks: 1,
    },
    address_funds_queries: DriveAbciQueryAddressFundsVersions {
        addresses_infos: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        address_info: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        addresses_trunk_state: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        addresses_branch_state: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        recent_address_balance_changes: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        recent_compacted_address_balance_changes: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
    },
};
