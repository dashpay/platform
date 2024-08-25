use crate::drive::identity::withdrawals::paths::WITHDRAWAL_TRANSACTIONS_NEXT_INDEX_KEY;
use crate::drive::identity::withdrawals::WithdrawalTransactionIndex;
use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::Error;
use grovedb::{Element, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn fetch_next_withdrawal_transaction_index_v0(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<WithdrawalTransactionIndex, Error> {
        let element = self
            .grove
            .get(
                &[Into::<&[u8; 1]>::into(RootTree::WithdrawalTransactions).as_slice()],
                &WITHDRAWAL_TRANSACTIONS_NEXT_INDEX_KEY,
                transaction,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        let Element::Item(counter_bytes, _) = element else {
            return Err(Error::Drive(
                DriveError::CorruptedWithdrawalTransactionsCounterNotItem(
                    "withdrawal transactions counter must be an item",
                ),
            ));
        };

        let counter =
            WithdrawalTransactionIndex::from_be_bytes(counter_bytes.try_into().map_err(|_| {
                DriveError::CorruptedWithdrawalTransactionsCounterInvalidLength(
                    "withdrawal transactions counter must be an u64",
                )
            })?);

        Ok(counter)
    }
}
