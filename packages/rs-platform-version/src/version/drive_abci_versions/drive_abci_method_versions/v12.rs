use super::v11::DRIVE_ABCI_METHOD_VERSIONS_V11;
use super::{DriveAbciIdentityCreditWithdrawalMethodVersions, DriveAbciMethodVersions};

/// Drive ABCI method versions 12, introduced in protocol v16. Differs from v11 in
/// `withdrawals.pool_withdrawals_into_transactions_queue`, which is 3 rather than 2: among the
/// oldest queued withdrawals (`SystemLimits::withdrawal_pooling_candidates_per_block_limit`) it
/// passes over one that does not fit what the withdrawal limits have left and pools the ones
/// behind it, where version 2 stops at the first withdrawal that does not fit. Protocol version
/// 16 raises the largest withdrawal (`SYSTEM_LIMITS_V6.max_withdrawal_amount`) above the floor
/// of the Core-anchored limit, so a withdrawal can be larger than that limit ever gets while
/// Core's credit pool is small; under version 2 it would hold every withdrawal queued behind it
/// until the pool grew.
pub const DRIVE_ABCI_METHOD_VERSIONS_V12: DriveAbciMethodVersions = DriveAbciMethodVersions {
    withdrawals: DriveAbciIdentityCreditWithdrawalMethodVersions {
        pool_withdrawals_into_transactions_queue: 3, // changed in v16: passes over a queued withdrawal that does not fit the limits and pools the ones behind it
        ..DRIVE_ABCI_METHOD_VERSIONS_V11.withdrawals
    },
    ..DRIVE_ABCI_METHOD_VERSIONS_V11
};
