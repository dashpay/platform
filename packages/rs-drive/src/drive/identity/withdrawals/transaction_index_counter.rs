use grovedb::{Element, TransactionArg};

use crate::{
    drive::{batch::GroveDbOpBatch, Drive, RootTree},
    error::{drive::DriveError, Error},
};

use super::paths::WITHDRAWAL_TRANSACTIONS_COUNTER_ID;

impl Drive {
    /// Get latest withdrawal index in a queue
    pub fn fetch_latest_withdrawal_transaction_index(
        &self,
        transaction: TransactionArg,
    ) -> Result<u64, Error> {
        let result = self
            .grove
            .get(
                [Into::<&[u8; 1]>::into(RootTree::WithdrawalTransactions).as_slice()],
                &WITHDRAWAL_TRANSACTIONS_COUNTER_ID,
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB);

        if let Err(Error::GroveDB(grovedb::Error::PathKeyNotFound(_))) = &result {
            return Ok(0);
        }

        let element = result?;

        if let Element::Item(counter_bytes, _) = element {
            let counter = u64::from_be_bytes(counter_bytes.try_into().map_err(|_| {
                DriveError::CorruptedWithdrawalTransactionsCounterInvalidLength(
                    "withdrawal transactions counter must be an u64",
                )
            })?);

            Ok(counter)
        } else {
            Err(Error::Drive(
                DriveError::CorruptedWithdrawalTransactionsCounterNotItem(
                    "withdrawal transactions counter must be an item",
                ),
            ))
        }
    }

    /// Add counter update operations to the batch
    pub fn add_update_withdrawal_index_counter_operation(
        &self,
        batch: &mut GroveDbOpBatch,
        value: Vec<u8>,
    ) {
        batch.add_insert(
            vec![vec![RootTree::WithdrawalTransactions as u8]],
            WITHDRAWAL_TRANSACTIONS_COUNTER_ID.to_vec(),
            Element::Item(value, None),
        );
    }
}
