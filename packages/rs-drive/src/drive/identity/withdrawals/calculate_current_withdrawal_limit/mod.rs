use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::fee::Credits;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

mod v0;
mod v1;

/// The withdrawal limit: how much may be pooled against how much already is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WithdrawalLimitInfo {
    /// The most that may be pooled and outstanding at once
    pub daily_maximum: Credits,
    /// What is already counted against it: the reservations of the last day up to protocol
    /// version 13, the withdrawals in flight from 14
    pub withdrawals_amount: Credits,
}

impl WithdrawalLimitInfo {
    /// What may still be pooled
    pub fn available(&self) -> Credits {
        self.daily_maximum.saturating_sub(self.withdrawals_amount)
    }
}

/// Core's credit pool as its `getcreditpoolinfo` RPC reports it at the block's chain locked
/// height, in credits. Every validator reads the same chain locked block, so the values are
/// deterministic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CoreCreditPoolSnapshot {
    /// The pool balance after the chain locked block
    pub balance: Credits,
    /// The pool balance after the block one window (Core's `CreditPoolPeriodBlocks`) earlier;
    /// 0 when that block predates the pool
    pub window_start_balance: Credits,
    /// The total of asset unlocks Core admits in the block after the chain locked one
    pub current_limit: Credits,
}

impl Drive {
    /// Calculates the withdrawal limit. From protocol version 14 (`calculate_current_withdrawal_limit`
    /// method version 1) it is derived from Core's credit pool, so `core_credit_pool` must be
    /// given there; earlier versions ignore it.
    pub fn calculate_current_withdrawal_limit(
        &self,
        core_credit_pool: Option<&CoreCreditPoolSnapshot>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<WithdrawalLimitInfo, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .withdrawals
            .calculate_current_withdrawal_limit
        {
            0 => self.calculate_current_withdrawal_limit_v0(transaction, platform_version),
            1 => {
                let core_credit_pool =
                    core_credit_pool.ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                        "calculate_current_withdrawal_limit v1 needs Core's credit pool",
                    )))?;
                self.calculate_current_withdrawal_limit_v1(
                    core_credit_pool,
                    transaction,
                    platform_version,
                )
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "calculate_current_withdrawal_limit".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
