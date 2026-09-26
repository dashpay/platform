use crate::version::drive_versions::drive_token_method_versions::{
    DriveTokenDistributionMethodVersions, DriveTokenFetchMethodVersions,
    DriveTokenLifecycleMethodVersions, DriveTokenMethodVersions, DriveTokenProveMethodVersions,
    DriveTokenUpdateMethodVersions,
};

/// Drive token methods for the 5.0 protocol version (17): the per-issuer
/// token lifecycle ledger.
///
/// Differences from [`super::v1::DRIVE_TOKEN_METHOD_VERSIONS_V1`]:
///
/// * `update.create_token_trees` 0 -> 1, `update.add_to_token_total_supply`
///   0 -> 1 and `update.remove_from_token_total_supply` 0 -> 1: every native
///   supply write keeps the issuer's supply rollup current in the same batch
///   and refuses a wiped issuer.
/// * `update.mint` 0 -> 1, `update.burn` 0 -> 1 and `update.mint_many` 0 -> 1:
///   the batch accumulated so far is handed to the supply writer, so several
///   supply writes for tokens of one issuer in one batch leave one replacement
///   of the issuer's record instead of one per token.
/// * `calculate_total_tokens_balance` 0 -> 1: the block end conservation check
///   reads the destroyed supply ledger and checks raw and active totals.
/// * `lifecycle`: every slot turns on at generation 0.
pub const DRIVE_TOKEN_METHOD_VERSIONS_V2: DriveTokenMethodVersions = DriveTokenMethodVersions {
    fetch: DriveTokenFetchMethodVersions {
        identity_token_balance: 0,
        identity_token_balances: 0,
        identities_token_balances: 0,
        identity_token_info: 0,
        identity_token_infos: 0,
        identities_token_infos: 0,
        token_statuses: 0,
        token_status: 0,
        token_total_supply: 0,
        token_total_aggregated_identity_balances: 0,
        pre_programmed_distributions: 0,
        perpetual_distribution_last_paid_time: 0,
        pre_programmed_distribution_last_paid_time: 0,
        token_direct_purchase_price: 0,
        token_direct_purchase_prices: 0,
        token_contract_info: 0,
    },
    prove: DriveTokenProveMethodVersions {
        identity_token_balance: 0,
        identity_token_balances: 0,
        identities_token_balances: 0,
        identity_token_info: 0,
        identity_token_infos: 0,
        identities_token_infos: 0,
        token_statuses: 0,
        total_supply_and_aggregated_identity_balances: 0,
        pre_programmed_distributions: 0,
        token_direct_purchase_prices: 0,
        perpetual_distribution_last_paid_time: 0,
        token_contract_info: 0,
    },
    update: DriveTokenUpdateMethodVersions {
        create_token_trees: 1, // changed in v2: creates the issuer's lifecycle record
        burn: 1, // changed in v2: hands the batch to the supply writer so the issuer record is written once
        mint: 1, // changed in v2: hands the batch to the supply writer so the issuer record is written once
        mint_many: 1, // changed in v2: hands the batch to the supply writer so the issuer record is written once
        transfer: 0,
        add_to_token_total_supply: 1, // changed in v2: moves the issuer rollup with the supply
        remove_from_token_total_supply: 1, // changed in v2: moves the issuer rollup with the supply
        remove_from_identity_token_balance: 0,
        add_to_identity_token_balance: 0,
        add_transaction_history_operations: 0,
        freeze: 0,
        unfreeze: 0,
        apply_status: 0,
        perpetual_distribution_next_event_for_identity_id: 0,
        set_direct_purchase_price: 0,
    },
    calculate_total_tokens_balance: 1, // changed in v2: reads the destroyed supply ledger
    distribution: DriveTokenDistributionMethodVersions {
        add_perpetual_distribution: 0,
        add_pre_programmed_distributions: 0,
        mark_perpetual_release_as_distributed: 0,
        mark_pre_programmed_release_as_distributed: 0,
    },
    lifecycle: DriveTokenLifecycleMethodVersions {
        fetch_contract_token_lifecycle: Some(0),
        fetch_token_lifecycles: Some(0),
        add_to_contract_issued_supply: Some(0),
        destroy_token_issuer: Some(0),
        add_estimation_costs_for_token_contract_lifecycles: Some(0),
    },
};
