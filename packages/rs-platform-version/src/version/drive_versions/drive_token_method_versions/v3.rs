use crate::version::drive_versions::drive_token_method_versions::v2::DRIVE_TOKEN_METHOD_VERSIONS_V2;
use crate::version::drive_versions::drive_token_method_versions::{
    DriveTokenMethodVersions, DriveTokenOrderEscrowMethodVersions,
};

/// Drive token methods for protocol v15+.
///
/// Identical to [`super::v2::DRIVE_TOKEN_METHOD_VERSIONS_V2`] except
/// `calculate_total_tokens_balance` is bumped to `2`: the tokens the token marketplace holds in
/// escrow for open listings join the identity balances and the shielded pools on the balance
/// side of the token conservation check, and `order_escrow.fetch_frozen_listings` is `Some(0)`:
/// `TokenDestroyFrozenFunds` destroys what a frozen identity's open listings hold too.
pub const DRIVE_TOKEN_METHOD_VERSIONS_V3: DriveTokenMethodVersions = DriveTokenMethodVersions {
    calculate_total_tokens_balance: 2,
    order_escrow: DriveTokenOrderEscrowMethodVersions {
        fetch_frozen_listings: Some(0),
        ..DRIVE_TOKEN_METHOD_VERSIONS_V2.order_escrow
    },
    ..DRIVE_TOKEN_METHOD_VERSIONS_V2
};
