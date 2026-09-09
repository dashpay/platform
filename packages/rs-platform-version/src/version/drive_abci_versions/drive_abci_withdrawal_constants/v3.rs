use crate::version::drive_abci_versions::drive_abci_withdrawal_constants::DriveAbciWithdrawalConstants;

/// Withdrawal constants for protocol version 14 and above: identical to
/// [`super::v2::DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V2`] plus `total_credits_history_prune_limit`,
/// bounding how many stale entries the per-block total credits history (the base of the
/// day-lagged daily withdrawal limit) drops per block.
pub const DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V3: DriveAbciWithdrawalConstants =
    DriveAbciWithdrawalConstants {
        core_expiration_blocks: 48,
        cleanup_expired_locks_of_withdrawal_amounts_limit: 64,
        total_credits_history_prune_limit: 64,
    };
