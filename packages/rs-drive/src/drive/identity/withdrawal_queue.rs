// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! This module defines functions within the Drive struct related to withdrawal transaction (AssetUnlock)
//!

use std::ops::RangeFull;

use grovedb::query_result_type::QueryResultType::QueryKeyElementPairResultType;
use grovedb::{Element, PathQuery, Query, QueryItem, SizedQuery, TransactionArg};

use crate::drive::batch::GroveDbOpBatch;
use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::DriveOperation;

/// constant id for transaction counter
pub const WITHDRAWAL_TRANSACTIONS_COUNTER_ID: [u8; 1] = [0];
/// constant id for subtree containing transactions queue
pub const WITHDRAWAL_TRANSACTIONS_QUEUE_ID: [u8; 1] = [1];

type WithdrawalTransaction = (Vec<u8>, Vec<u8>);

/// Add operations for creating initial withdrawal state structure
pub fn add_initial_withdrawal_state_structure_operations(batch: &mut GroveDbOpBatch) {
    batch.add_insert_empty_tree(vec![], vec![RootTree::WithdrawalTransactions as u8]);

    batch.add_insert(
        vec![vec![RootTree::WithdrawalTransactions as u8]],
        WITHDRAWAL_TRANSACTIONS_COUNTER_ID.to_vec(),
        Element::Item(0u64.to_be_bytes().to_vec(), None),
    );

    batch.add_insert_empty_tree(
        vec![vec![RootTree::WithdrawalTransactions as u8]],
        WITHDRAWAL_TRANSACTIONS_QUEUE_ID.to_vec(),
    );
}

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

    /// Add insert operations for withdrawal transactions to the batch
    pub fn add_enqueue_withdrawal_transaction_operations(
        &self,
        batch: &mut GroveDbOpBatch,
        withdrawals: Vec<(Vec<u8>, Vec<u8>)>,
    ) {
        for (id, bytes) in withdrawals {
            batch.add_insert(
                vec![
                    vec![RootTree::WithdrawalTransactions as u8],
                    WITHDRAWAL_TRANSACTIONS_QUEUE_ID.to_vec(),
                ],
                id,
                Element::Item(bytes, None),
            );
        }
    }

    /// Get specified amount of withdrawal transactions from the DB
    pub fn dequeue_withdrawal_transactions(
        &self,
        num_of_transactions: u16,
        transaction: TransactionArg,
    ) -> Result<Vec<WithdrawalTransaction>, Error> {
        let mut query = Query::new();

        query.insert_item(QueryItem::RangeFull(RangeFull));

        let path_query = PathQuery {
            path: vec![
                vec![RootTree::WithdrawalTransactions as u8],
                WITHDRAWAL_TRANSACTIONS_QUEUE_ID.to_vec(),
            ],
            query: SizedQuery {
                query,
                limit: Some(num_of_transactions),
                offset: None,
            },
        };

        let result_items = self
            .grove
            .query_raw(&path_query, QueryKeyElementPairResultType, transaction)
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
            let mut batch_operations: Vec<DriveOperation> = vec![];
            let mut drive_operations: Vec<DriveOperation> = vec![];

            let withdrawals_path: [&[u8]; 2] = [
                Into::<&[u8; 1]>::into(RootTree::WithdrawalTransactions),
                &WITHDRAWAL_TRANSACTIONS_QUEUE_ID,
            ];

            for (id, _) in withdrawals.iter() {
                self.batch_delete(
                    withdrawals_path,
                    id,
                    true,
                    true,
                    transaction,
                    &mut batch_operations,
                )?;
            }

            self.apply_batch_drive_operations(
                true,
                transaction,
                batch_operations,
                &mut drive_operations,
            )?;
        }

        Ok(withdrawals)
    }
}

#[cfg(test)]
mod tests {
    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;
    use crate::drive::batch::GroveDbOpBatch;

    mod queue {
        use super::*;

        #[test]
        fn test_enqueue_and_dequeue() {
            let drive = setup_drive_with_initial_state_structure();

            let transaction = drive.grove.start_transaction();

            let withdrawals: Vec<(Vec<u8>, Vec<u8>)> = (0..17)
                .map(|i: u8| (i.to_be_bytes().to_vec(), vec![i; 32]))
                .collect();

            let mut batch = GroveDbOpBatch::new();

            drive.add_enqueue_withdrawal_transaction_operations(&mut batch, withdrawals);

            drive
                .grove_apply_batch(batch, true, Some(&transaction))
                .expect("to apply ops");

            let withdrawals = drive
                .dequeue_withdrawal_transactions(16, Some(&transaction))
                .expect("to dequeue withdrawals");

            assert_eq!(withdrawals.len(), 16);

            let withdrawals = drive
                .dequeue_withdrawal_transactions(16, Some(&transaction))
                .expect("to dequeue withdrawals");

            assert_eq!(withdrawals.len(), 1);

            let withdrawals = drive
                .dequeue_withdrawal_transactions(16, Some(&transaction))
                .expect("to dequeue withdrawals");

            assert_eq!(withdrawals.len(), 0);
        }
    }

    mod index {
        use super::*;

        #[test]
        fn test_withdrawal_transaction_counter() {
            let drive = setup_drive_with_initial_state_structure();

            let transaction = drive.grove.start_transaction();

            let mut batch = GroveDbOpBatch::new();

            let counter: u64 = 42;

            drive.add_update_withdrawal_index_counter_operation(
                &mut batch,
                counter.to_be_bytes().to_vec(),
            );

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
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
    }
}
