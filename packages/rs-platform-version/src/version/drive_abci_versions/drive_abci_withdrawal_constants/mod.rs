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
    /// Maximum number of Core blocks `scan_core_blocks_for_withdrawals` reads per Platform
    /// block. When the chain lock height jumps further, the rest is read in the blocks after
    /// (`0` disables the scan; protocol versions before 14 have no scan).
    pub core_blocks_scanned_per_block_limit: u16,
}
