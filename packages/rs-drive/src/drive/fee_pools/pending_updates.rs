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
use crate::fee::refunds::CreditsPerEpoch;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, TransactionArg};

type PendingUpdatesCount = usize;

impl Drive {
    /// Fetches existing pending epoch pool updates using specified epochs
    /// and returns merged result
    pub fn fetch_and_merge_with_existing_pending_epoch_pool_updates(
        &self,
        mut credits_per_epoch: CreditsPerEpoch,
        transaction: TransactionArg,
    ) -> Result<CreditsPerEpoch, Error> {
        let mut query = Query::new();

        for (epoch_index_key, _) in credits_per_epoch.iter() {
            let epoch_index: u64 = epoch_index_key.to_owned();
            let encoded_epoch_index = epoch_index.to_be_bytes().to_vec();

            query.insert_key(encoded_epoch_index);
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
        for (encoded_epoch_index, element) in query_result.to_key_elements() {
            let epoch_index =
                u16::from_be_bytes(encoded_epoch_index.as_slice().try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(
                        "epoch index for pending pool updates must be u64",
                    ))
                })?);

            let epoch_index_key = epoch_index as u64;

            let Some(credits_to_update) = credits_per_epoch.get(epoch_index_key) else {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution("pending updates should contain fetched epochs")));
            };

            let Element::Item(encoded_existing_signed_credits, _) = element else {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution("pending updates should contain only items")));
            };

            let existing_signed_credits = i64::from_be_bytes(
                encoded_existing_signed_credits
                    .as_slice()
                    .try_into()
                    .map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(
                            "credits for pending pool updates must be i64",
                        ))
                    })?,
            );

            let existing_credits = existing_signed_credits as u64;

            credits_per_epoch.insert(epoch_index_key, credits_to_update + existing_credits);
        }

        Ok(credits_per_epoch)
    }

    /// Adds operations to delete pending epoch pool updates except specified epochs
    pub fn add_delete_pending_epoch_pool_updates_except_specified_operations(
        &self,
        batch: &mut GroveDbOpBatch,
        credits_per_epoch: &CreditsPerEpoch,
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

        for (encoded_epoch_index, _) in query_result.to_key_elements() {
            let epoch_index =
                u16::from_be_bytes(encoded_epoch_index.as_slice().try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(
                        "epoch index for pending pool updates must be i64",
                    ))
                })?);

            let epoch_index_key = epoch_index as u64;

            if credits_per_epoch.contains_key(epoch_index_key) {
                continue;
            }

            batch.add_delete(pools_pending_updates_path(), encoded_epoch_index);
        }

        Ok(())
    }
}

/// Returns the index of the unpaid Epoch.
pub fn add_update_pending_epoch_pool_update_operations(
    batch: &mut GroveDbOpBatch,
    credits_per_epoch: CreditsPerEpoch,
) -> Result<PendingUpdatesCount, Error> {
    let credits_per_epoch_count = credits_per_epoch.len();

    for (epoch_index_key, credits) in credits_per_epoch {
        let epoch_index = epoch_index_key as u16;
        let encoded_epoch_index = epoch_index.to_be_bytes().to_vec();

        let signed_credits = -(credits as i64);

        let element = Element::new_item(signed_credits.to_be_bytes().to_vec());

        batch.add_insert(pools_pending_updates_path(), encoded_epoch_index, element);
    }

    Ok(credits_per_epoch_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;

    mod fetch_and_merge_with_existing_pending_epoch_pool_updates {
        use super::*;
    }

    mod add_delete_pending_epoch_pool_updates_except_specified_operations {
        use super::*;
    }

    mod add_update_pending_epoch_pool_update_operations {
        use super::*;

        mod helpers {
            use super::*;
            use grovedb::batch::Op;

            pub fn process_and_assert_pending_updates(
                drive: &Drive,
                pending_updates: CreditsPerEpoch,
                encoded_pending_updates: &HashMap<Vec<u8>, i64>,
                transaction: TransactionArg,
            ) {
                let pending_updates_count = pending_updates.len();

                let mut batch = GroveDbOpBatch::new();

                let updates_count = drive
                    .add_update_pending_epoch_pool_update_operations(
                        &mut batch,
                        pending_updates,
                        transaction,
                    )
                    .expect("should update pending pool updates");

                drive
                    .grove_apply_batch(batch.clone(), false, transaction)
                    .expect("should apply batch");

                assert_eq!(batch.len(), updates_count);
                assert_eq!(batch.len(), pending_updates_count);

                for operation in batch.operations {
                    assert_eq!(operation.path.to_path(), pools_pending_updates_path());

                    let encoded_key = operation.key.get_key();

                    assert!(encoded_pending_updates.contains_key(&encoded_key));

                    let expected_credits = encoded_pending_updates[&encoded_key];

                    let expected_encoded_credits = expected_credits.to_be_bytes().to_vec();

                    let Op::Insert {
                        element: Element::Item(encoded_credits, None)
                    } = operation.op else {
                        panic!("pending pool update should be stored as an item");
                    };

                    assert_eq!(encoded_credits, expected_encoded_credits);
                }
            }
        }

        #[test]
        fn should_add_new_pending_pool_updates() {
            let drive = setup_drive_with_initial_state_structure();

            let transaction = drive.grove.start_transaction();

            // Add initial set of pending updates

            let initial_pending_updates =
                CreditsPerEpoch::from_iter(vec![(1, 15), (3, 25), (7, 95), (9, 100), (12, 120)]);

            let initial_encoded_epoch_indices_and_signed_credits = initial_pending_updates
                .clone()
                .into_iter()
                .map(|(epoch_index, credits)| {
                    let epoch_index_key = epoch_index as u16;
                    let epoch_index_key = epoch_index_key.to_be_bytes().to_vec();

                    // Credits must be stored as negative to support total credits balance
                    // verification with sum trees
                    (epoch_index_key, -(credits as i64))
                })
                .collect::<HashMap<_, _>>();

            helpers::process_and_assert_pending_updates(
                &drive,
                initial_pending_updates,
                &initial_encoded_epoch_indices_and_signed_credits,
                Some(&transaction),
            );

            // Add new and update existing

            let additional_pending_updates =
                CreditsPerEpoch::from_iter(vec![(1, 15), (3, 25), (15, 95), (16, 100)]);

            let additional_encoded_epoch_indices_and_signed_credits = additional_pending_updates
                .clone()
                .into_iter()
                .map(|(epoch_index, credits)| {
                    let epoch_index_key = epoch_index as u16;
                    let epoch_index_key = epoch_index_key.to_be_bytes().to_vec();
                    let mut signed_credits = -(credits as i64);

                    if let Some(initial_signed_credits) =
                        initial_encoded_epoch_indices_and_signed_credits.get(&epoch_index_key)
                    {
                        signed_credits += initial_signed_credits;
                    }

                    // Credits must be stored as negative to support total credits balance
                    // verification with sum trees
                    (epoch_index_key, signed_credits)
                })
                .collect::<HashMap<_, _>>();

            helpers::process_and_assert_pending_updates(
                &drive,
                additional_pending_updates,
                &additional_encoded_epoch_indices_and_signed_credits,
                Some(&transaction),
            );
        }
    }
}
