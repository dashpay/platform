use crate::drive::fee_pools::pools_path;
use crate::drive::Drive;
use grovedb::{Element, TransactionArg};

use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee_pools::epochs_root_tree_key_constants::KEY_STORAGE_FEE_POOL;

impl Drive {
    pub fn get_aggregate_storage_fees_from_distribution_pool(
        &self,
        transaction: TransactionArg,
    ) -> Result<u64, Error> {
        let element = self
            .grove
            .get(pools_path(), KEY_STORAGE_FEE_POOL.as_slice(), transaction)
            .unwrap()
            .map_err(Error::GroveDB)?;

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
        fn test_error_if_pool_is_not_initiated() {
            let drive = setup_drive(None);
            let transaction = drive.grove.start_transaction();

            match drive.get_aggregate_storage_fees_from_distribution_pool(Some(&transaction)) {
                Ok(_) => assert!(
                    false,
                    "should not be able to get genesis time on uninit fee pools"
                ),
                Err(e) => match e {
                    Error::GroveDB(grovedb::Error::PathNotFound(_)) => assert!(true),
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
    }
}
