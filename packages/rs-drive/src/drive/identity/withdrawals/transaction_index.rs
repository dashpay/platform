use grovedb::{Element, TransactionArg};

use crate::drive::identity::withdrawals::WithdrawalTransactionIndex;
use crate::{
    drive::{
        batch::{drive_op_batch::WithdrawalOperationType, DriveOperation},
        Drive, RootTree,
    },
    error::{drive::DriveError, Error},
};

use super::paths::WITHDRAWAL_TRANSACTIONS_NEXT_INDEX_KEY;

impl Drive {
    /// Fetches next withdrawal transaction index
    pub fn fetch_next_withdrawal_transaction_index(
        &self,
        transaction: TransactionArg,
    ) -> Result<WithdrawalTransactionIndex, Error> {
        let element = self
            .grove
            .get(
                &[Into::<&[u8; 1]>::into(RootTree::WithdrawalTransactions).as_slice()],
                &WITHDRAWAL_TRANSACTIONS_NEXT_INDEX_KEY,
                transaction,
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

    /// Add next transaction index increment operation to the batch
    pub fn add_update_next_withdrawal_transaction_index_operation(
        &self,
        index: WithdrawalTransactionIndex,
        drive_operation_types: &mut Vec<DriveOperation>,
    ) {
        drive_operation_types.push(DriveOperation::WithdrawalOperation(
            WithdrawalOperationType::UpdateIndexCounter { index },
        ));
    }
}

#[cfg(test)]
mod tests {
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::version::PlatformVersion;

    use crate::tests::helpers::setup::setup_drive_with_initial_state_structure;

    #[test]
    fn test_next_withdrawal_transaction_index() {
        let drive = setup_drive_with_initial_state_structure();

        let platform_version = PlatformVersion::latest();

        let transaction = drive.grove.start_transaction();

        let block_info = BlockInfo {
            time_ms: 1,
            height: 1,
            core_height: 1,
            epoch: Epoch::new(1).unwrap(),
        };

        let mut batch = vec![];

        let counter: u64 = 42;

        drive.add_update_next_withdrawal_transaction_index_operation(counter, &mut batch);

        drive
            .apply_drive_operations(
                batch,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
            )
            .expect("to apply drive ops");

        let stored_counter = drive
            .fetch_next_withdrawal_transaction_index(Some(&transaction))
            .expect("to withdraw counter");

        assert_eq!(stored_counter, counter);
    }

    #[test]
    fn test_initial_withdrawal_transaction_index() {
        let drive = setup_drive_with_initial_state_structure();

        let transaction = drive.grove.start_transaction();

        let stored_counter = drive
            .fetch_next_withdrawal_transaction_index(Some(&transaction))
            .expect("to withdraw counter");

        assert_eq!(stored_counter, 0);
    }
}
