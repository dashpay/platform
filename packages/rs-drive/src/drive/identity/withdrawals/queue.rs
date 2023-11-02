use std::ops::RangeFull;

use grovedb::{
    query_result_type::QueryResultType, Element, PathQuery, Query, QueryItem, SizedQuery,
    TransactionArg,
};

use crate::drive::identity::withdrawals::WithdrawalTransactionIdAndBytes;
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
    pub fn add_enqueue_withdrawal_transaction_operations<'a>(
        &self,
        withdrawals: &'a [WithdrawalTransactionIdAndBytes],
        drive_operation_types: &mut Vec<DriveOperation<'a>>,
    ) {
        if !withdrawals.is_empty() {
            drive_operation_types.push(DriveOperation::WithdrawalOperation(
                WithdrawalOperationType::InsertTransactions {
                    withdrawal_transactions: withdrawals,
                },
            ));
        }
    }

    /// Get specified amount of withdrawal transactions from the DB
    pub fn dequeue_withdrawal_transactions(
        &self,
        max_amount: u16,
        transaction: TransactionArg,
        drive_operation_types: &mut Vec<DriveOperation>,
    ) -> Result<Vec<WithdrawalTransactionIdAndBytes>, Error> {
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

        let withdrawals = result_items
            .into_iter()
            .map(|(id, element)| match element {
                Element::Item(bytes, _) => Ok((id, bytes)),
                _ => Err(Error::Drive(DriveError::CorruptedWithdrawalNotItem(
                    "withdrawal is not an item",
                ))),
            })
            .collect::<Result<Vec<(Vec<u8>, Vec<u8>)>, Error>>()?;

        if !withdrawals.is_empty() {
            for (id, _) in withdrawals.iter() {
                drive_operation_types.push(DriveOperation::WithdrawalOperation(
                    WithdrawalOperationType::DeleteWithdrawalTransaction { id: id.clone() },
                ));
            }
        }

        Ok(withdrawals)
    }
}

#[cfg(test)]
mod tests {
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

        let withdrawals: Vec<(Vec<u8>, Vec<u8>)> = (0..17)
            .map(|i: u8| (i.to_be_bytes().to_vec(), vec![i; 32]))
            .collect();

        let block_info = BlockInfo {
            time_ms: 1,
            height: 1,
            core_height: 1,
            epoch: Epoch::new(1).unwrap(),
        };

        let mut drive_operations: Vec<DriveOperation> = vec![];

        drive.add_enqueue_withdrawal_transaction_operations(&withdrawals, &mut drive_operations);

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
            .dequeue_withdrawal_transactions(16, Some(&transaction), &mut drive_operations)
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
            .dequeue_withdrawal_transactions(16, Some(&transaction), &mut drive_operations)
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
            .dequeue_withdrawal_transactions(16, Some(&transaction), &mut drive_operations)
            .expect("to dequeue withdrawals");

        assert_eq!(drive_operations.len(), 0);
    }
}
