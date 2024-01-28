use grovedb::{Element, TransactionArg};

use crate::{
    drive::{
        batch::{drive_op_batch::WithdrawalOperationType, DriveOperation},
        Drive, RootTree,
    },
    error::{drive::DriveError, Error},
};

use super::paths::WITHDRAWAL_TRANSACTIONS_INDEX_COUNTER_KEY;

impl Drive {
    /// Fetches latest withdrawal index in the transactions queue
    pub fn fetch_latest_withdrawal_transaction_index(
        &self,
        transaction: TransactionArg,
    ) -> Result<u64, Error> {
        let element = self
            .grove
            .get(
                &[Into::<&[u8; 1]>::into(RootTree::WithdrawalTransactions).as_slice()],
                &WITHDRAWAL_TRANSACTIONS_INDEX_COUNTER_KEY,
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

        let counter = u64::from_be_bytes(counter_bytes.try_into().map_err(|_| {
            DriveError::CorruptedWithdrawalTransactionsCounterInvalidLength(
                "withdrawal transactions counter must be an u64",
            )
        })?);

        Ok(counter)
    }

    /// Add counter update operations to the batch
    pub fn add_update_withdrawal_index_counter_operation(
        &self,
        value: u64,
        drive_operation_types: &mut Vec<DriveOperation>,
    ) {
        drive_operation_types.push(DriveOperation::WithdrawalOperation(
            WithdrawalOperationType::UpdateIndexCounter { index: value },
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
    fn test_withdrawal_transaction_counter() {
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

        drive.add_update_withdrawal_index_counter_operation(counter, &mut batch);

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
            .fetch_latest_withdrawal_transaction_index(Some(&transaction))
            .expect("to withdraw counter");

        assert_eq!(stored_counter, counter);
    }

    #[test]
    fn test_returns_0_if_empty() {
        let drive = setup_drive_with_initial_state_structure();

        let transaction = drive.grove.start_transaction();

        let stored_counter = drive
            .fetch_latest_withdrawal_transaction_index(Some(&transaction))
            .expect("to withdraw counter");

        assert_eq!(stored_counter, 0);
    }
}
