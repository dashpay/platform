use crate::version::drive_abci_versions::drive_abci_withdrawal_constants::DriveAbciWithdrawalConstants;

pub const DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V1: DriveAbciWithdrawalConstants = DriveAbciWithdrawalConstants {
    core_expiration_blocks: 48,
    cleanup_expired_locks_of_withdrawal_amounts_limit: 0,
};