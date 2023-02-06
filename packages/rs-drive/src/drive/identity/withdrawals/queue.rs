use std::ops::RangeFull;

use grovedb::{
    query_result_type::QueryResultType, Element, PathQuery, Query, QueryItem, SizedQuery,
    TransactionArg,
};

use crate::{
    drive::{
        batch::{drive_op_batch::WithdrawalOperationType, DriveOperationType},
        Drive,
    },
    error::{drive::DriveError, Error},
};

use super::paths::{get_withdrawal_transactions_queue_path_vec, WithdrawalTransaction};

impl Drive {
    /// Add insert operations for withdrawal transactions to the batch
    pub fn add_enqueue_withdrawal_transaction_operations<'a>(
        &self,
        withdrawals: &'a [WithdrawalTransaction],
        drive_operation_types: &mut Vec<DriveOperationType<'a>>,
    ) {
        if !withdrawals.is_empty() {
            drive_operation_types.push(DriveOperationType::WithdrawalOperation(
                WithdrawalOperationType::InsertTransactions {
                    transactions: withdrawals,
                },
            ));
        }
    }

    /// Get specified amount of withdrawal transactions from the DB
    pub fn dequeue_withdrawal_transactions<'a>(
        &self,
        max_amount: u16,
        transaction: TransactionArg,
        drive_operation_types: &mut Vec<DriveOperationType<'a>>,
    ) -> Result<Vec<WithdrawalTransaction>, Error> {
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
                drive_operation_types.push(DriveOperationType::WithdrawalOperation(
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
        drive::{batch::DriveOperationType, block_info::BlockInfo},
        fee_pools::epochs::Epoch,
        tests::helpers::setup::setup_drive_with_initial_state_structure,
    };

    #[test]
    fn test_enqueue_and_dequeue() {
        let drive = setup_drive_with_initial_state_structure();

        let transaction = drive.grove.start_transaction();

        let withdrawals: Vec<(Vec<u8>, Vec<u8>)> = (0..17)
            .map(|i: u8| (i.to_be_bytes().to_vec(), vec![i; 32]))
            .collect();

        let block_info = BlockInfo {
            time_ms: 1,
            height: 1,
            epoch: Epoch::new(1),
        };

        let mut drive_operations: Vec<DriveOperationType> = vec![];

        drive.add_enqueue_withdrawal_transaction_operations(&withdrawals, &mut drive_operations);

        drive
            .apply_drive_operations(drive_operations, true, &block_info, Some(&transaction))
            .expect("to apply batch");

        let mut drive_operations: Vec<DriveOperationType> = vec![];

        let withdrawals = drive
            .dequeue_withdrawal_transactions(16, Some(&transaction), &mut drive_operations)
            .expect("to dequeue withdrawals");

        drive
            .apply_drive_operations(drive_operations, true, &block_info, Some(&transaction))
            .expect("to apply batch");

        assert_eq!(withdrawals.len(), 16);

        let mut drive_operations: Vec<DriveOperationType> = vec![];

        let withdrawals = drive
            .dequeue_withdrawal_transactions(16, Some(&transaction), &mut drive_operations)
            .expect("to dequeue withdrawals");

        drive
            .apply_drive_operations(drive_operations, true, &block_info, Some(&transaction))
            .expect("to apply batch");

        assert_eq!(withdrawals.len(), 1);

        let mut drive_operations: Vec<DriveOperationType> = vec![];

        let withdrawals = drive
            .dequeue_withdrawal_transactions(16, Some(&transaction), &mut drive_operations)
            .expect("to dequeue withdrawals");

        drive
            .apply_drive_operations(drive_operations, true, &block_info, Some(&transaction))
            .expect("to apply batch");

        assert_eq!(withdrawals.len(), 0);
    }
}
