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

//! Pending epoch pool updates
//!
//! Credit refunds are calculated when data is removed for the state.
//! Identity is refunded immediately when we update Identity balance
//! after state transition execution, but epoch pools must be updated
//! as well to deduct refunded amount. To do not update every block all
//! storage epoch pools, we introduce additional structure which aggregate
//! all pending refunds for epochs and apply them during
//! storage fee distribution on epoch change.
//!

use crate::drive::batch::GroveDbOpBatch;
use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::credits::Creditable;
use crate::fee::epoch::CreditsPerEpoch;
use crate::fee::get_overflow_error;
use crate::fee_pools::epochs_root_tree_key_constants::KEY_PENDING_EPOCH_REFUNDS;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, TransactionArg};

/// Returns the path to pending epoch refunds
pub fn pending_epoch_refunds_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Pools as u8],
        KEY_PENDING_EPOCH_REFUNDS.to_vec(),
    ]
}

impl Drive {
    /// Fetches all pending epoch refunds
    pub fn fetch_pending_epoch_refunds(
        &self,
        transaction: TransactionArg,
    ) -> Result<CreditsPerEpoch, Error> {
        let mut query = Query::new();

        query.insert_all();

        let (query_result, _) = self
            .grove
            .query_raw(
                &PathQuery::new_unsized(pending_epoch_refunds_path_vec(), query),
                transaction.is_some(),
                QueryResultType::QueryKeyElementPairResultType,
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        query_result
            .to_key_elements()
            .into_iter()
            .map(|(epoch_index_key, element)| {
                let epoch_index =
                    u16::from_be_bytes(epoch_index_key.as_slice().try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(
                            "epoch index for pending pool updates must be i64",
                        ))
                    })?);

                if let Element::SumItem(credits, _) = element {
                    Ok((epoch_index, credits.to_unsigned()))
                } else {
                    Err(Error::Drive(DriveError::CorruptedCodeExecution(
                        "pending refund credits must be sum items",
                    )))
                }
            })
            .collect::<Result<CreditsPerEpoch, Error>>()
    }

    /// Fetches pending epoch refunds and adds them to specified collection
    pub fn fetch_and_add_pending_epoch_refunds_to_collection(
        &self,
        mut refunds_per_epoch: CreditsPerEpoch,
        transaction: TransactionArg,
    ) -> Result<CreditsPerEpoch, Error> {
        if refunds_per_epoch.is_empty() {
            return Ok(refunds_per_epoch);
        }

        let mut query = Query::new();

        for epoch_index in refunds_per_epoch.keys() {
            let epoch_index_key = epoch_index.to_be_bytes().to_vec();

            query.insert_key(epoch_index_key);
        }

        // Query existing pending updates
        let (query_result, _) = self
            .grove
            .query_raw(
                &PathQuery::new_unsized(pending_epoch_refunds_path_vec(), query),
                transaction.is_some(),
                QueryResultType::QueryKeyElementPairResultType,
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        // Merge with existing pending updates
        for (epoch_index_key, element) in query_result.to_key_elements() {
            let epoch_index =
                u16::from_be_bytes(epoch_index_key.as_slice().try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(
                        "epoch index for pending epoch refunds must be u16",
                    ))
                })?);

            let existing_credits = refunds_per_epoch.get_mut(&epoch_index).ok_or(Error::Drive(
                DriveError::CorruptedCodeExecution(
                    "pending epoch refunds should contain fetched epochs",
                ),
            ))?;

            if let Element::SumItem(credits, _) = element {
                *existing_credits = existing_credits
                    .checked_add(credits.to_unsigned())
                    .ok_or_else(|| get_overflow_error("pending epoch refunds credits overflow"))?;
            } else {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "pending epoch refunds credits must be sum items",
                )));
            }
        }

        Ok(refunds_per_epoch)
    }

    /// Adds operations to delete pending epoch refunds except epochs from provided collection
    pub fn add_delete_pending_epoch_refunds_except_specified_operations(
        &self,
        batch: &mut GroveDbOpBatch,
        refunds_per_epoch: &CreditsPerEpoch,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        // TODO: Replace with key iterator
        let mut query = Query::new();

        query.insert_all();

        let (query_result, _) = self
            .grove
            .query_raw(
                &PathQuery::new_unsized(pending_epoch_refunds_path_vec(), query),
                transaction.is_some(),
                QueryResultType::QueryKeyElementPairResultType,
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        for (epoch_index_key, _) in query_result.to_key_elements() {
            let epoch_index =
                u16::from_be_bytes(epoch_index_key.as_slice().try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(
                        "pending updates epoch index for must be u16",
                    ))
                })?);

            if refunds_per_epoch.contains_key(&epoch_index) {
                continue;
            }

            batch.add_delete(pending_epoch_refunds_path_vec(), epoch_index_key);
        }

        Ok(())
    }
}

/// Adds GroveDB batch operations to update pending epoch storage pool updates
pub fn add_update_pending_epoch_refunds_operations(
    batch: &mut GroveDbOpBatch,
    refunds_per_epoch: CreditsPerEpoch,
) -> Result<(), Error> {
    for (epoch_index, credits) in refunds_per_epoch {
        let epoch_index_key = epoch_index.to_be_bytes().to_vec();

        let element = Element::new_sum_item(-credits.to_signed()?);

        batch.add_insert(pending_epoch_refunds_path_vec(), epoch_index_key, element);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;

    mod fetch_and_add_pending_epoch_refunds_to_collection {
        use super::*;

        #[test]
        fn should_fetch_and_merge_pending_updates() {
            let drive = setup_drive_with_initial_state_structure();

            let transaction = drive.grove.start_transaction();

            // Store initial set of pending refunds

            let initial_pending_refunds =
                CreditsPerEpoch::from_iter([(1, 15), (3, 25), (7, 95), (9, 100), (12, 120)]);

            let mut batch = GroveDbOpBatch::new();

            add_update_pending_epoch_refunds_operations(&mut batch, initial_pending_refunds)
                .expect("should update pending epoch updates");

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            // Fetch and merge

            let new_pending_refunds =
                CreditsPerEpoch::from_iter([(1, 15), (3, 25), (30, 195), (41, 150)]);

            let updated_pending_refunds = drive
                .fetch_and_add_pending_epoch_refunds_to_collection(
                    new_pending_refunds,
                    Some(&transaction),
                )
                .expect("should fetch and merge pending updates");

            let expected_pending_refunds =
                CreditsPerEpoch::from_iter([(1, 30), (3, 50), (30, 195), (41, 150)]);

            assert_eq!(updated_pending_refunds, expected_pending_refunds);
        }
    }

    mod add_delete_pending_epoch_refunds_except_specified_operations {
        use super::*;
        use grovedb::batch::Op;

        #[test]
        fn should_add_delete_operations() {
            let drive = setup_drive_with_initial_state_structure();

            let transaction = drive.grove.start_transaction();

            // Store initial set of pending refunds

            let initial_pending_refunds =
                CreditsPerEpoch::from_iter([(1, 15), (3, 25), (7, 95), (9, 100), (12, 120)]);

            let mut batch = GroveDbOpBatch::new();

            add_update_pending_epoch_refunds_operations(&mut batch, initial_pending_refunds)
                .expect("should update pending epoch updates");

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            // Delete existing pending refunds except specified epochs

            let new_pending_refunds = CreditsPerEpoch::from_iter([(1, 15), (3, 25)]);

            let mut batch = GroveDbOpBatch::new();

            drive
                .add_delete_pending_epoch_refunds_except_specified_operations(
                    &mut batch,
                    &new_pending_refunds,
                    Some(&transaction),
                )
                .expect("should fetch and merge pending updates");

            let expected_pending_refunds =
                CreditsPerEpoch::from_iter([(7, 95), (9, 100), (12, 120)]);

            assert_eq!(batch.len(), expected_pending_refunds.len());

            for operation in batch.into_iter() {
                assert!(matches!(operation.op, Op::Delete));

                assert_eq!(operation.path.to_path(), pending_epoch_refunds_path_vec());

                let epoch_index_key = operation.key.get_key();
                let epoch_index = u16::from_be_bytes(
                    epoch_index_key
                        .try_into()
                        .expect("should convert to u16 bytes"),
                );

                assert!(expected_pending_refunds.contains_key(&epoch_index));
            }
        }
    }
}
