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

//! Storage Fee Distribution Pool.
//!

use crate::drive::fee_pools::pools_path;
use crate::drive::Drive;
use grovedb::{Element, TransactionArg};

use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee_pools::epochs_root_tree_key_constants::KEY_STORAGE_FEE_POOL;

impl Drive {
    /// Returns the amount of credits in the storage fee distribution pool.
    pub fn get_aggregate_storage_fees_from_distribution_pool(
        &self,
        transaction: TransactionArg,
    ) -> Result<u64, Error> {
        match self
            .grove
            .get(pools_path(), KEY_STORAGE_FEE_POOL.as_slice(), transaction)
            .unwrap()
        {
            Ok(element) => {
                if let Element::Item(item, _) = element {
                    let fee = u64::from_be_bytes(item.as_slice().try_into().map_err(|_| {
                        Error::Fee(FeeError::CorruptedStorageFeePoolInvalidItemLength(
                            "fee pools storage fee pool is not i64",
                        ))
                    })?);

                    Ok(fee)
                } else {
                    Err(Error::Fee(FeeError::CorruptedStorageFeePoolNotItem(
                        "fee pools storage fee pool must be an item",
                    )))
                }
            }
            Err(grovedb::Error::PathKeyNotFound(_)) => Ok(0),
            Err(e) => Err(Error::GroveDB(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    mod get_aggregate_storage_fees_from_distribution_pool {
        use crate::common::helpers::setup::{
            setup_drive, setup_drive_with_initial_state_structure,
        };
        use crate::drive::batch::GroveDbOpBatch;
        use crate::drive::fee_pools::pools_vec_path;
        use crate::error::fee::FeeError;
        use crate::error::Error;
        use crate::fee_pools::epochs_root_tree_key_constants::KEY_STORAGE_FEE_POOL;
        use grovedb::Element;

        #[test]
        fn test_error_if_epoch_is_not_initiated() {
            let drive = setup_drive(None);
            let transaction = drive.grove.start_transaction();

            match drive.get_aggregate_storage_fees_from_distribution_pool(Some(&transaction)) {
                Ok(_) => assert!(
                    false,
                    "should not be able to get genesis time on uninit fee pools"
                ),
                Err(e) => match e {
                    Error::GroveDB(grovedb::Error::PathParentLayerNotFound(_)) => assert!(true),
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_error_if_wrong_value_encoded() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let mut batch = GroveDbOpBatch::new();

            batch.add_insert(
                pools_vec_path(),
                KEY_STORAGE_FEE_POOL.to_vec(),
                Element::Item(u128::MAX.to_be_bytes().to_vec(), None),
            );

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            match drive.get_aggregate_storage_fees_from_distribution_pool(Some(&transaction)) {
                Ok(_) => assert!(false, "should not be able to decode stored value"),
                Err(e) => match e {
                    Error::Fee(FeeError::CorruptedStorageFeePoolInvalidItemLength(_)) => {
                        assert!(true)
                    }
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_error_if_storage_pool_is_not_initiated() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            // Remove storage pool key such as we would init the epoch
            // with `add_init_empty_without_storage_operations` method
            let mut batch = GroveDbOpBatch::new();

            batch.add_delete(pools_vec_path(), KEY_STORAGE_FEE_POOL.to_vec());

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let result = drive
                .get_aggregate_storage_fees_from_distribution_pool(Some(&transaction))
                .expect("should get aggregated storage fees");

            assert_eq!(result, 0);
        }
    }
}
