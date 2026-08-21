use crate::version::drive_versions::drive_group_method_versions::{
    DriveAddressFundsCostEstimationMethodVersions, DriveAddressFundsMethodVersions,
};

/// V2 differs from V1 only in `cost_estimation.for_address_balance_update`,
/// which bumps `0 → 1` to switch the CLEAR_ADDRESS_POOL leaf-layer
/// estimation from `EstimatedLayerSizes::AllItems(...)` to
/// `AllItemsWithSumItem(...)`. The v0 method's output is byte-stable for
/// v11 (still selected via `DRIVE_VERSION_V6` → V1 table); v1 only
/// becomes the active estimation at protocol v12+ where shielded pool
/// + sum-tree feature land. Unblocked by grovedb #674.
pub const DRIVE_ADDRESS_FUNDS_METHOD_VERSIONS_V2: DriveAddressFundsMethodVersions =
    DriveAddressFundsMethodVersions {
        set_balance_to_address: 0,
        add_balance_to_address: 0,
        remove_balance_from_address: 0,
        fetch_balance_and_nonce: 0,
        fetch_balances_with_nonces: 0,
        prove_balance_and_nonce: 0,
        prove_balances_with_nonces: 0,
        prove_address_funds_trunk_query: 0,
        prove_address_funds_branch_query: 0,
        address_funds_query_min_depth: 6,
        address_funds_query_max_depth: 9,
        estimate_funding_fee: 0,
        cost_estimation: DriveAddressFundsCostEstimationMethodVersions {
            for_address_balance_update: 1,
        },
    };
