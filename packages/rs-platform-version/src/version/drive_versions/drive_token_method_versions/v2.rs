use crate::version::drive_versions::drive_token_method_versions::{
    DriveTokenDistributionMethodVersions, DriveTokenFetchMethodVersions, DriveTokenMethodVersions,
    DriveTokenProveMethodVersions, DriveTokenUpdateMethodVersions,
};

// Introduced in protocol version 14 with token shielded pools: `calculate_total_tokens_balance`
// moves to 1 so the per-token pool balances (a BigSumTree sibling of the identity balances
// under the Tokens root) count on the balance side of the token conservation check. The
// pool creation and shield / unshield / shielded-transfer slots are all 0 at introduction.

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
        create_token_trees: 0,
        burn: 0,
        mint: 0,
        mint_many: 0,
        transfer: 0,
        add_to_token_total_supply: 0,
        remove_from_token_total_supply: 0,
        remove_from_identity_token_balance: 0,
        add_to_identity_token_balance: 0,
        add_transaction_history_operations: 0,
        freeze: 0,
        unfreeze: 0,
        apply_status: 0,
        perpetual_distribution_next_event_for_identity_id: 0,
        create_token_shielded_pool_trees: 0,
        shield: 0,
        unshield: 0,
        shielded_transfer: 0,
    },
    calculate_total_tokens_balance: 1, // changed: token shielded pool balances join the identity balances on the conservation side
    distribution: DriveTokenDistributionMethodVersions {
        add_perpetual_distribution: 0,
        add_pre_programmed_distributions: 0,
        mark_perpetual_release_as_distributed: 0,
        mark_pre_programmed_release_as_distributed: 0,
    },
};
