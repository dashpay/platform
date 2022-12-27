use grovedb::{
    query_result_type::{QueryResultElement, QueryResultType},
    Element, PathQuery, Query, SizedQuery, TransactionArg,
};

use crate::{
    drive::{
        batch::drive_op_batch::{DriveOperationConverter, WithdrawalOperationType},
        block_info::BlockInfo,
        Drive, RootTree,
    },
    error::{drive::DriveError, Error},
    fee::op::DriveOperation,
};

use super::paths::{
    get_withdrawal_transactions_expired_ids_path,
    get_withdrawal_transactions_expired_ids_path_as_u8, WITHDRAWAL_TRANSACTIONS_COUNTER_ID,
};

impl Drive {
    /// Get latest withdrawal index in a queue
    pub fn fetch_latest_withdrawal_transaction_index(
        &self,
        transaction: TransactionArg,
    ) -> Result<u64, Error> {
        let mut inner_query = Query::new();

        inner_query.insert_all();

        let expired_index_query = PathQuery::new(
            get_withdrawal_transactions_expired_ids_path(),
            SizedQuery::new(inner_query, Some(1), None),
        );

        let (expired_index_elements, _) = self
            .grove
            .query_raw(
                &expired_index_query,
                QueryResultType::QueryKeyElementPairResultType,
                transaction,
            )
            .unwrap()?;

        if !expired_index_elements.is_empty() {
            let expired_index_element_pair = expired_index_elements.elements.get(0).unwrap();

            if let QueryResultElement::KeyElementPairResultItem((key, _)) =
                expired_index_element_pair
            {
                let index = u64::from_be_bytes(key.clone().try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "Transaction index has wrong length",
                    ))
                })?);

                let path: [&[u8]; 2] = get_withdrawal_transactions_expired_ids_path_as_u8();

                self.grove.delete(path, key, None, transaction).unwrap()?;

                return Ok(index);
            }
        }

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
        value: u64,
        block_info: &BlockInfo,
        drive_operations: &mut Vec<DriveOperation>,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        drive_operations.extend(
            WithdrawalOperationType::UpdateIndexCounter { index: value }.to_drive_operations(
                self,
                &mut None,
                block_info,
                transaction,
            )?,
        );

        Ok(())

        // batch.add_insert(
        //     vec![vec![RootTree::WithdrawalTransactions as u8]],
        //     WITHDRAWAL_TRANSACTIONS_COUNTER_ID.to_vec(),
        //     Element::Item(value, None),
        // );
    }

    /// Add insert expired counter operations
    pub fn add_insert_expired_index_operation(
        &self,
        transaction_index: u64,
        block_info: &BlockInfo,
        drive_operations: &mut Vec<DriveOperation>,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        let operations = WithdrawalOperationType::InsertExpiredIndex {
            index: transaction_index,
        }
        .to_drive_operations(self, &mut None, block_info, transaction)?;

        drive_operations.extend(operations);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use grovedb::Element;

    use crate::{
        common::helpers::setup::setup_drive_with_initial_state_structure,
        drive::{
            block_info::BlockInfo,
            identity::withdrawals::paths::get_withdrawal_transactions_expired_ids_path_as_u8,
        },
        fee_pools::epochs::Epoch,
    };

    #[test]
    fn test_withdrawal_transaction_counter() {
        let drive = setup_drive_with_initial_state_structure();

        let transaction = drive.grove.start_transaction();

        let block_info = BlockInfo {
            time_ms: 1,
            height: 1,
            epoch: Epoch::new(1),
        };

        let mut batch = vec![];
        let mut result_operations = vec![];

        let counter: u64 = 42;

        drive
            .add_update_withdrawal_index_counter_operation(
                counter,
                &block_info,
                &mut batch,
                Some(&transaction),
            )
            .expect("to add update operations");

        drive
            .apply_batch_drive_operations(None, Some(&transaction), batch, &mut result_operations)
            .expect("to apply ops");

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

    #[test]
    fn test_should_return_expired_index_if_any() {
        let drive = setup_drive_with_initial_state_structure();

        let transaction = drive.grove.start_transaction();

        let bytes = 42u64.to_be_bytes();

        let path = get_withdrawal_transactions_expired_ids_path_as_u8();

        drive
            .grove
            .insert(
                path,
                &bytes,
                Element::Item(bytes.to_vec(), None),
                None,
                Some(&transaction),
            )
            .unwrap()
            .expect("to update index counter");

        let stored_counter = drive
            .fetch_latest_withdrawal_transaction_index(Some(&transaction))
            .expect("to withdraw counter");

        assert_eq!(stored_counter, 42);
    }
}
