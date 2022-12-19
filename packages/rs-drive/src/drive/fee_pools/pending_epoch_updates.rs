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

use crate::drive::batch::GroveDbOpBatch;
use crate::drive::fee_pools::pools_pending_updates_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::credits::{Creditable, SignedCredits};
use crate::fee::epoch::SignedCreditsPerEpoch;
use crate::fee::get_overflow_error;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, TransactionArg};

impl Drive {
    /// Fetches all pending epoch pool updates
    pub fn fetch_pending_updates(
        &self,
        transaction: TransactionArg,
    ) -> Result<SignedCreditsPerEpoch, Error> {
        let mut query = Query::new();

        query.insert_all();

        let (query_result, _) = self
            .grove
            .query_raw(
                &PathQuery::new_unsized(pools_pending_updates_path(), query),
                QueryResultType::QueryKeyElementPairResultType,
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        query_result.to_key_elements().into_iter().map(|(epoch_index_key, element)| {
            let epoch_index =
                u16::from_be_bytes(epoch_index_key.as_slice().try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(
                        "epoch index for pending pool updates must be i64",
                    ))
                })?);

            let Element::SumItem(..) = element else {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution("pending updates credits must be sum items")));
            };

            let credits: SignedCredits = element.sum_value().ok_or(
            Error::Drive(DriveError::CorruptedCodeExecution("pending updates credits must have value")
            ))?;

            Ok((epoch_index, credits))
        }).collect::<Result<SignedCreditsPerEpoch, Error>>()
    }

    /// Fetches existing pending epoch pool updates using specified epochs
    /// and returns merged result
    pub fn fetch_and_merge_with_existing_pending_epoch_storage_pool_updates(
        &self,
        mut credits_per_epoch: SignedCreditsPerEpoch,
        transaction: TransactionArg,
    ) -> Result<SignedCreditsPerEpoch, Error> {
        if credits_per_epoch.is_empty() {
            return Ok(credits_per_epoch);
        }

        let mut query = Query::new();

        for epoch_index in credits_per_epoch.keys() {
            let epoch_index_key = epoch_index.to_be_bytes().to_vec();

            query.insert_key(epoch_index_key);
        }

        // Query existing pending updates
        let (query_result, _) = self
            .grove
            .query_raw(
                &PathQuery::new_unsized(pools_pending_updates_path(), query),
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
                        "epoch index for pending pool updates must be u16",
                    ))
                })?);

            let Some(credits_to_update) = credits_per_epoch.get(&epoch_index) else {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution("pending updates should contain fetched epochs")));
            };

            let Element::SumItem(..) = element else {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution("pending updates credits must be sum items")));
            };

            let existing_credits: SignedCredits =
                element
                    .sum_value()
                    .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                        "pending updates credits must have value",
                    )))?;

            let result_credits = credits_to_update
                .checked_add(existing_credits)
                .ok_or_else(|| get_overflow_error("pending updates credits overflow"))?;

            credits_per_epoch.insert(epoch_index, result_credits);
        }

        Ok(credits_per_epoch)
    }

    /// Adds operations to delete pending epoch pool updates except specified epochs
    pub fn add_delete_pending_epoch_storage_pool_updates_except_specified_operations(
        &self,
        batch: &mut GroveDbOpBatch,
        credits_per_epoch: &SignedCreditsPerEpoch,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        // TODO: Replace with key iterator
        let mut query = Query::new();

        query.insert_all();

        let (query_result, _) = self
            .grove
            .query_raw(
                &PathQuery::new_unsized(pools_pending_updates_path(), query),
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

            if credits_per_epoch.contains_key(&epoch_index) {
                continue;
            }

            batch.add_delete(pools_pending_updates_path(), epoch_index_key);
        }

        Ok(())
    }
}

/// Adds GroveDB batch operations to update pending epoch storage pool updates
pub fn add_update_pending_epoch_storage_pool_update_operations(
    batch: &mut GroveDbOpBatch,
    credits_per_epoch: SignedCreditsPerEpoch,
) -> Result<(), Error> {
    for (epoch_index, credits) in credits_per_epoch {
        let epoch_index_key = epoch_index.to_be_bytes().to_vec();

        let element = Element::new_sum_item(credits.to_signed()?);

        batch.add_insert(pools_pending_updates_path(), epoch_index_key, element);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;

    mod fetch_and_merge_with_existing_pending_epoch_storage_pool_updates {
        use super::*;

        #[test]
        fn should_fetch_and_merge_pending_updates() {
            let drive = setup_drive_with_initial_state_structure();

            let transaction = drive.grove.start_transaction();

            // Store initial set of pending updates

            let initial_pending_updates =
                SignedCreditsPerEpoch::from_iter([(1, 15), (3, 25), (7, 95), (9, 100), (12, 120)]);

            let mut batch = GroveDbOpBatch::new();

            add_update_pending_epoch_storage_pool_update_operations(
                &mut batch,
                initial_pending_updates,
            )
            .expect("should update pending epoch updates");

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            // Fetch and merge

            let new_pending_updates =
                SignedCreditsPerEpoch::from_iter([(1, 15), (3, 25), (30, 195), (41, 150)]);

            let updated_pending_updates = drive
                .fetch_and_merge_with_existing_pending_epoch_storage_pool_updates(
                    new_pending_updates,
                    Some(&transaction),
                )
                .expect("should fetch and merge pending updates");

            let expected_pending_updates =
                SignedCreditsPerEpoch::from_iter([(1, 30), (3, 50), (30, 195), (41, 150)]);

            assert_eq!(updated_pending_updates, expected_pending_updates);
        }
    }

    mod add_delete_pending_epoch_storage_pool_updates_except_specified_operations {
        use super::*;
        use grovedb::batch::Op;

        #[test]
        fn should_add_delete_operations() {
            let drive = setup_drive_with_initial_state_structure();

            let transaction = drive.grove.start_transaction();

            // Store initial set of pending updates

            let initial_pending_updates =
                SignedCreditsPerEpoch::from_iter([(1, 15), (3, 25), (7, 95), (9, 100), (12, 120)]);

            let mut batch = GroveDbOpBatch::new();

            add_update_pending_epoch_storage_pool_update_operations(
                &mut batch,
                initial_pending_updates,
            )
            .expect("should update pending epoch updates");

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            // Delete existing pending updates expect specified pending updates

            let new_pending_updates = SignedCreditsPerEpoch::from_iter([(1, 15), (3, 25)]);

            let mut batch = GroveDbOpBatch::new();

            drive
                .add_delete_pending_epoch_storage_pool_updates_except_specified_operations(
                    &mut batch,
                    &new_pending_updates,
                    Some(&transaction),
                )
                .expect("should fetch and merge pending updates");

            let expected_pending_updates =
                SignedCreditsPerEpoch::from_iter([(7, 95), (9, 100), (12, 120)]);

            assert_eq!(batch.len(), expected_pending_updates.len());

            for operation in batch.into_iter() {
                assert!(matches!(operation.op, Op::Delete));

                assert_eq!(operation.path.to_path(), pools_pending_updates_path());

                let epoch_index_key = operation.key.get_key();
                let epoch_index = u16::from_be_bytes(
                    epoch_index_key
                        .try_into()
                        .expect("should convert to u16 bytes"),
                );

                assert!(expected_pending_updates.contains_key(&epoch_index));
            }
        }
    }
}
