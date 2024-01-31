use std::ops::RangeFull;

use grovedb::{
    query_result_type::QueryResultType, Element, PathQuery, Query, QueryItem, SizedQuery,
    TransactionArg,
};

use crate::drive::identity::withdrawals::{
    WithdrawalTransactionIndex, WithdrawalTransactionIndexAndBytes,
};
use crate::{
    drive::{
        batch::{drive_op_batch::WithdrawalOperationType, DriveOperation},
        Drive,
    },
    error::{drive::DriveError, Error},
};

use super::paths::get_withdrawal_transactions_queue_path_vec;

impl Drive {
    /// Add insert operations for withdrawal transactions to the batch
    pub fn add_enqueue_untied_withdrawal_transaction_operations<'a>(
        &self,
        withdrawal_transactions: Vec<WithdrawalTransactionIndexAndBytes>,
        drive_operation_types: &mut Vec<DriveOperation<'a>>,
    ) {
        if !withdrawal_transactions.is_empty() {
            drive_operation_types.push(DriveOperation::WithdrawalOperation(
                WithdrawalOperationType::InsertTransactions {
                    withdrawal_transactions,
                },
            ));
        }
    }

    /// Get specified amount of withdrawal transactions from the DB
    pub fn dequeue_untied_withdrawal_transactions(
        &self,
        max_amount: u16,
        transaction: TransactionArg,
        drive_operation_types: &mut Vec<DriveOperation>,
    ) -> Result<Vec<WithdrawalTransactionIndexAndBytes>, Error> {
        let mut query = Query::new();

        query.insert_item(QueryItem::RangeFull(RangeFull));

        let path_query = PathQuery {
            path: get_withdrawal_transactions_queue_path_vec(),
            query: SizedQuery {
                query,
                limit: Some(max_amount),
                offset: None,
            },
        };

        let result_items = self
            .grove
            .query_raw(
                &path_query,
                transaction.is_some(),
                true,
                QueryResultType::QueryKeyElementPairResultType,
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?
            .0
            .to_key_elements();

        let withdrawal_transactions = result_items
            .into_iter()
            .map(|(index_bytes, element)| match element {
                Element::Item(bytes, _) => {
                    let index = WithdrawalTransactionIndex::from_be_bytes(
                        index_bytes.try_into().map_err(|_| {
                            Error::Drive(DriveError::CorruptedSerialization(String::from(
                                "withdrawal index must be u64",
                            )))
                        })?,
                    );

                    Ok((index, bytes))
                }
                _ => Err(Error::Drive(DriveError::CorruptedWithdrawalNotItem(
                    "withdrawal is not an item",
                ))),
            })
            .collect::<Result<Vec<WithdrawalTransactionIndexAndBytes>, Error>>()?;

        if !withdrawal_transactions.is_empty() {
            for (index, _) in withdrawal_transactions.iter() {
                drive_operation_types.push(DriveOperation::WithdrawalOperation(
                    WithdrawalOperationType::DeleteWithdrawalTransaction { index: *index },
                ));
            }
        }

        Ok(withdrawal_transactions)
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::identity::withdrawals::{
        WithdrawalTransactionIndex, WithdrawalTransactionIndexAndBytes,
    };
    use crate::{
        drive::batch::DriveOperation,
        tests::helpers::setup::setup_drive_with_initial_state_structure,
    };
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::version::PlatformVersion;

    #[test]
    fn test_enqueue_and_dequeue() {
        let drive = setup_drive_with_initial_state_structure();

        let platform_version = PlatformVersion::latest();

        let transaction = drive.grove.start_transaction();

        let withdrawals: Vec<WithdrawalTransactionIndexAndBytes> = (0..17)
            .map(|i: u8| (i as WithdrawalTransactionIndex, vec![i; 32]))
            .collect();

        let block_info = BlockInfo {
            time_ms: 1,
            height: 1,
            core_height: 1,
            epoch: Epoch::new(1).unwrap(),
        };

        let mut drive_operations: Vec<DriveOperation> = vec![];

        drive.add_enqueue_untied_withdrawal_transaction_operations(
            &withdrawals,
            &mut drive_operations,
        );

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
            )
            .expect("to apply batch");

        let mut drive_operations: Vec<DriveOperation> = vec![];

        let withdrawals = drive
            .dequeue_untied_withdrawal_transactions(16, Some(&transaction), &mut drive_operations)
            .expect("to dequeue withdrawals");

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
            )
            .expect("to apply batch");

        assert_eq!(withdrawals.len(), 16);

        let mut drive_operations: Vec<DriveOperation> = vec![];

        let withdrawals = drive
            .dequeue_untied_withdrawal_transactions(16, Some(&transaction), &mut drive_operations)
            .expect("to dequeue withdrawals");

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
            )
            .expect("to apply batch");

        assert_eq!(withdrawals.len(), 1);

        let mut drive_operations: Vec<DriveOperation> = vec![];

        drive
            .dequeue_untied_withdrawal_transactions(16, Some(&transaction), &mut drive_operations)
            .expect("to dequeue withdrawals");

        assert_eq!(drive_operations.len(), 0);
    }
}
