use crate::version::drive_versions::drive_group_method_versions::{
    DriveAddressFundsCostEstimationMethodVersions, DriveAddressFundsMethodVersions,
};

pub const DRIVE_ADDRESS_FUNDS_METHOD_VERSIONS_V1: DriveAddressFundsMethodVersions =
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
        address_funds_query_min_depth: 4,
        address_funds_query_max_depth: 4,
        cost_estimation: DriveAddressFundsCostEstimationMethodVersions {
            for_address_balance_update: 0,
        },
    };
