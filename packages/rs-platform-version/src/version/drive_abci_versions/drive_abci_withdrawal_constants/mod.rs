pub mod v1;
pub mod v2;
pub mod v3;

#[derive(Clone, Debug, Default)]
pub struct DriveAbciWithdrawalConstants {
    pub core_expiration_blocks: u32,
    pub cleanup_expired_locks_of_withdrawal_amounts_limit: u16,
    /// Maximum number of entries `record_total_credits_history_for_withdrawals` prunes from
    /// the total credits history per block (`0` disables pruning).
    pub total_credits_history_prune_limit: u16,
}
