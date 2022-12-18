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

use crate::drive::batch::GroveDbOpBatch;
use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::credits::{Creditable, SignedCredits};
use crate::fee::epoch::SignedCreditsPerEpoch;
use crate::fee::get_overflow_error;
use crate::fee_pools::epochs::epoch_key_constants::KEY_POOL_STORAGE_FEES;
use crate::fee_pools::epochs::{paths, Epoch};
use crate::fee_pools::epochs_root_tree_key_constants::{
    KEY_PENDING_POOL_UPDATES, KEY_STORAGE_FEE_POOL,
};
use grovedb::{Element, PathQuery, Query, TransactionArg};
use itertools::Itertools;

/// Epochs module
pub mod epochs;
pub mod pending_epoch_updates;
pub mod storage_fee_distribution_pool;
pub mod unpaid_epoch;

/// Returns the path to the Pools subtree.
pub fn pools_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::Pools)]
}

/// Returns the path to the Pools subtree as a mutable vector.
pub fn pools_vec_path() -> Vec<Vec<u8>> {
    vec![vec![RootTree::Pools as u8]]
}

/// Returns the path to pending pool updates
pub fn pools_pending_updates_path() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Pools as u8],
        KEY_PENDING_POOL_UPDATES.to_vec(),
    ]
}

/// Returns the path to the aggregate storage fee distribution pool.
pub fn aggregate_storage_fees_distribution_pool_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Pools),
        KEY_STORAGE_FEE_POOL,
    ]
}

/// Returns the path to the aggregate storage fee distribution pool as a mutable vector.
pub fn aggregate_storage_fees_distribution_pool_vec_path() -> Vec<Vec<u8>> {
    vec![vec![RootTree::Pools as u8], KEY_STORAGE_FEE_POOL.to_vec()]
}

impl Drive {
    /// Adds GroveDB operations to update epoch storage fee pools with specified map of credits to epochs
    pub fn add_update_epoch_storage_fee_pools_operations(
        &self,
        batch: &mut GroveDbOpBatch,
        credits_per_epochs: SignedCreditsPerEpoch,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        if credits_per_epochs.is_empty() {
            return Ok(());
        }

        let min_epoch_index_key = credits_per_epochs.keys().min().ok_or(Error::Drive(
            DriveError::CorruptedCodeExecution("can't find min epoch index"),
        ))?;
        let min_epoch_index = min_epoch_index_key.to_owned() as u16;
        let min_encoded_epoch_index = paths::encode_epoch_index_key(min_epoch_index)?.to_vec();

        let max_epoch_index_key = credits_per_epochs.keys().max().ok_or(Error::Drive(
            DriveError::CorruptedCodeExecution("can't find max epoch index"),
        ))?;
        let max_epoch_index = max_epoch_index_key.to_owned() as u16;
        let max_encoded_epoch_index = paths::encode_epoch_index_key(max_epoch_index)?.to_vec();

        let mut storage_fee_pool_query = Query::new();
        storage_fee_pool_query.insert_key(KEY_POOL_STORAGE_FEES.to_vec());

        let mut epochs_query = Query::new();

        epochs_query.insert_range_inclusive(min_encoded_epoch_index..=max_encoded_epoch_index);

        epochs_query.set_subquery(storage_fee_pool_query);

        let (mut storage_fee_pools, _) = self
            .grove
            .query(
                &PathQuery::new_unsized(pools_vec_path(), epochs_query),
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        if storage_fee_pools.len() != credits_per_epochs.len() {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "number of fetched epoch storage fee pools must be equal to requested for update",
            )));
        }

        // Sort epochs to reverse order to pop existing values
        for (epoch_index, credits) in credits_per_epochs.into_iter().sorted_by_key(|x| !x.0) {
            let encoded_existing_storage_fee =
                storage_fee_pools
                    .pop()
                    .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                        "value must be present",
                    )))?;

            let existing_storage_fee = SignedCredits::from_vec_bytes(encoded_existing_storage_fee)?;

            let credits_to_update = existing_storage_fee.checked_add(credits).ok_or_else(|| {
                get_overflow_error("can't add credits to existing epoch pool storage fee")
            })?;

            let element = Element::new_item(credits_to_update.to_unsigned().to_vec_bytes());

            batch.add_insert(
                Epoch::new(epoch_index).get_vec_path(),
                KEY_POOL_STORAGE_FEES.to_vec(),
                element,
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;

    mod add_update_epoch_storage_fee_pools_operations {
        use super::*;
        use crate::fee::credits::Credits;
        use crate::fee::epoch::{EpochIndex, GENESIS_EPOCH_INDEX};
        use grovedb::batch::GroveDbOp;

        #[test]
        fn should_do_nothing_if_credits_per_epoch_are_empty() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let credits_per_epoch = SignedCreditsPerEpoch::default();

            let mut batch = GroveDbOpBatch::new();

            drive
                .add_update_epoch_storage_fee_pools_operations(
                    &mut batch,
                    credits_per_epoch,
                    Some(&transaction),
                )
                .expect("should update epoch storage pools");

            assert_eq!(batch.len(), 0);
        }

        #[test]
        fn should_update_epoch_storage_fee_pools() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            const TO_EPOCH_INDEX: EpochIndex = 1;

            // Store initial epoch storage pool values
            let operations = (GENESIS_EPOCH_INDEX..TO_EPOCH_INDEX)
                .map(|epoch_index| {
                    let credits = 10 - epoch_index as Credits;

                    let element = Element::new_item(credits.to_unsigned().to_vec_bytes());

                    GroveDbOp::insert_op(
                        Epoch::new(epoch_index).get_vec_path(),
                        KEY_POOL_STORAGE_FEES.to_vec(),
                        element,
                    )
                })
                .collect();

            let batch = GroveDbOpBatch::from_operations(operations);

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let credits_to_epochs: SignedCreditsPerEpoch = (GENESIS_EPOCH_INDEX..TO_EPOCH_INDEX)
                .map(|epoch_index| (epoch_index, epoch_index as SignedCredits))
                .collect();

            let mut batch = GroveDbOpBatch::new();

            drive
                .add_update_epoch_storage_fee_pools_operations(
                    &mut batch,
                    credits_to_epochs,
                    Some(&transaction),
                )
                .expect("should update epoch storage pools");

            assert_eq!(batch.len(), TO_EPOCH_INDEX as usize);

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
