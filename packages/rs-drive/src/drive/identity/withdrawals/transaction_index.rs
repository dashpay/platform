use grovedb::{
    query_result_type::{QueryResultElement, QueryResultType},
    Element, PathQuery, Query, SizedQuery, TransactionArg,
};

use crate::{
    drive::{
        batch::{drive_op_batch::WithdrawalOperationType, DriveOperation},
        Drive, RootTree,
    },
    error::{drive::DriveError, Error},
};

use super::paths::{
    get_withdrawal_transactions_expired_ids_path_vec, WITHDRAWAL_TRANSACTIONS_COUNTER_ID,
};

impl Drive {
    /// Get and remove latest withdrawal index in a queue
    pub fn fetch_and_remove_latest_withdrawal_transaction_index_operations(
        &self,
        drive_operation_types: &mut Vec<DriveOperation>,
        transaction: TransactionArg,
    ) -> Result<u64, Error> {
        // TODO(withdrawals): we don't need to reuse indices. Just use always incremental counter.
        let mut inner_query = Query::new();

        inner_query.insert_all();

        let expired_index_query = PathQuery::new(
            get_withdrawal_transactions_expired_ids_path_vec(),
            SizedQuery::new(inner_query, Some(1), None),
        );

        let (expired_index_elements, _) = self
            .grove
            .query_raw(
                &expired_index_query,
                transaction.is_some(),
                true,
                QueryResultType::QueryKeyElementPairResultType,
                transaction,
            )
            .unwrap()?;

        if !expired_index_elements.is_empty() {
            let expired_index_element_pair = expired_index_elements.elements.get(0).unwrap();

            if let QueryResultElement::KeyElementPairResultItem((key, _)) =
                expired_index_element_pair
            {
                drive_operation_types.push(DriveOperation::WithdrawalOperation(
                    WithdrawalOperationType::DeleteExpiredIndex { key: key.clone() },
                ));

                let index = u64::from_be_bytes(key.clone().try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "Transaction index has wrong length",
                    ))
                })?);

                return Ok(index);
            }
        }

        let result = self
            .grove
            .get(
                &[Into::<&[u8; 1]>::into(RootTree::WithdrawalTransactions).as_slice()],
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
        drive_operation_types: &mut Vec<DriveOperation>,
    ) {
        drive_operation_types.push(DriveOperation::WithdrawalOperation(
            WithdrawalOperationType::UpdateIndexCounter { index: value },
        ));
    }

    /// Add insert expired counter operations
    pub fn add_insert_expired_index_operation(
        &self,
        transaction_index: u64,
        drive_operation_types: &mut Vec<DriveOperation>,
    ) {
        drive_operation_types.push(DriveOperation::WithdrawalOperation(
            WithdrawalOperationType::InsertExpiredIndex {
                index: transaction_index,
            },
        ));
    }
}

#[cfg(test)]
mod tests {
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::version::PlatformVersion;
    use grovedb::Element;

    use crate::{
        drive::identity::withdrawals::paths::get_withdrawal_transactions_expired_ids_path,
        tests::helpers::setup::setup_drive_with_initial_state_structure,
    };

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

        let mut batch = vec![];

        let stored_counter = drive
            .fetch_and_remove_latest_withdrawal_transaction_index_operations(
                &mut batch,
                Some(&transaction),
            )
            .expect("to withdraw counter");

        drive
            .apply_drive_operations(
                batch,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
            )
            .expect("to apply drive ops");

        assert_eq!(stored_counter, counter);
    }

    #[test]
    fn test_returns_0_if_empty() {
        let drive = setup_drive_with_initial_state_structure();

        let transaction = drive.grove.start_transaction();

        let mut batch = vec![];

        let stored_counter = drive
            .fetch_and_remove_latest_withdrawal_transaction_index_operations(
                &mut batch,
                Some(&transaction),
            )
            .expect("to withdraw counter");

        assert_eq!(stored_counter, 0);
    }

    #[test]
    fn test_should_return_expired_index_if_any() {
        let drive = setup_drive_with_initial_state_structure();

        let transaction = drive.grove.start_transaction();

        let bytes = 42u64.to_be_bytes();

        let path = get_withdrawal_transactions_expired_ids_path();

        drive
            .grove
            .insert(
                &path,
                &bytes,
                Element::Item(bytes.to_vec(), None),
                None,
                Some(&transaction),
            )
            .unwrap()
            .expect("to update index counter");

        let mut batch = vec![];

        let stored_counter = drive
            .fetch_and_remove_latest_withdrawal_transaction_index_operations(
                &mut batch,
                Some(&transaction),
            )
            .expect("to withdraw counter");

        assert_eq!(stored_counter, 42);
    }
}
