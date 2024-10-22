pub mod v1;
pub mod v2;

#[derive(Clone, Debug, Default)]
pub struct DriveAbciWithdrawalConstants {
    pub core_expiration_blocks: u32,
    pub cleanup_expired_locks_of_withdrawal_amounts_limit: u16,
}