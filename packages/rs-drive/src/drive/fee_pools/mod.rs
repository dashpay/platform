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
use crate::fee::epoch::CreditsPerEpoch;
use crate::fee_pools::epochs::epoch_key_constants::KEY_POOL_STORAGE_FEES;
use crate::fee_pools::epochs::{paths, Epoch};
use crate::fee_pools::epochs_root_tree_key_constants::{
    KEY_PENDING_POOL_UPDATES, KEY_STORAGE_FEE_POOL,
};
use crate::query::QueryItem;
use grovedb::{PathQuery, Query, TransactionArg};
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
    pub fn add_update_epoch_storage_fee_pools_operations(
        &self,
        batch: &mut GroveDbOpBatch,
        credits_per_epochs: CreditsPerEpoch,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        if credits_per_epochs.len() == 0 {
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

        let (storage_fee_pools, _) = self
            .grove
            .query(
                &PathQuery::new_unsized(pools_vec_path(), epochs_query),
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        if storage_fee_pools.len() != credits_per_epochs.len() {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "number of fetched epoch  storage fee pools must be equal to requested for update",
            )));
        }

        for (i, (epoch_index_key, credits)) in credits_per_epochs
            .into_iter()
            .sorted_by_key(|x| x.0)
            .enumerate()
        {
            let encoded_epoch_index_key =
                paths::encode_epoch_index_key(epoch_index_key as u16)?.to_vec();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;
    use crate::error;
    use crate::fee_pools::epochs::Epoch;

    mod create_fee_pool_trees {
        #[test]
        fn test_values_are_set() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let storage_fee_pool = drive
                .get_aggregate_storage_fees_from_distribution_pool(Some(&transaction))
                .expect("should get storage fee pool");

            assert_eq!(storage_fee_pool, 0u64);
        }

        #[test]
        fn test_epoch_trees_are_created() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            for epoch_index in 0..1000 {
                let epoch = super::Epoch::new(epoch_index);

                let storage_fee = drive
                    .get_epoch_storage_credits_for_distribution(&epoch, Some(&transaction))
                    .expect("should get storage fee");

                assert_eq!(storage_fee, 0);
            }

            let epoch = super::Epoch::new(1000); // 1001th epochs pool

            match drive.get_epoch_storage_credits_for_distribution(&epoch, Some(&transaction)) {
                Ok(_) => assert!(false, "must be an error"),
                Err(e) => match e {
                    super::error::Error::GroveDB(_) => assert!(true),
                    _ => assert!(false, "invalid error type"),
                },
            }
        }
    }
}
