use crate::drive::identity::withdrawals::WithdrawalTransactionIndex;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

mod v0;

impl Drive {
    /// Fetches next withdrawal transaction index
    pub fn fetch_next_withdrawal_transaction_index(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<WithdrawalTransactionIndex, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .withdrawals
            .transaction
            .index
            .fetch_next_withdrawal_transaction_index
        {
            0 => self.fetch_next_withdrawal_transaction_index_v0(transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_next_withdrawal_transaction_index".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
