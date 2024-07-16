use crate::drive::credit_pools::paths::pools_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::balances::credits::Creditable;
use dpp::fee::Credits;
use grovedb::{Element, TransactionArg};
use platform_version::version::PlatformVersion;

use crate::drive::credit_pools::epochs::epochs_root_tree_key_constants::KEY_STORAGE_FEE_POOL;

impl Drive {
    /// Returns the amount of credits in the storage fee distribution pool.
    pub(super) fn get_storage_fees_from_distribution_pool_v0(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, Error> {
        match self
            .grove
            .get(
                &pools_path(),
                KEY_STORAGE_FEE_POOL.as_slice(),
                transaction,
                &platform_version.drive.grove_version,
            )
            .unwrap()
        {
            Ok(Element::SumItem(credits, _)) => Ok(credits.to_unsigned()),
            Ok(_) => Err(Error::Drive(DriveError::UnexpectedElementType(
                "fee pools storage fee pool must be sum item",
            ))),
            Err(grovedb::Error::PathKeyNotFound(_)) => Ok(0),
            Err(e) => Err(Error::GroveDB(e)),
        }
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use super::*;

    use crate::util::test_helpers::setup::{setup_drive, setup_drive_with_initial_state_structure};

    mod get_storage_fees_from_distribution_pool {
        use super::*;
        use crate::drive::credit_pools::paths::pools_vec_path;
        use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
        use crate::util::batch::GroveDbOpBatch;
        use dpp::version::PlatformVersion;

        #[test]
        fn test_error_if_epoch_is_not_initiated() {
            let drive = setup_drive(None);
            let transaction = drive.grove.start_transaction();

            let platform_version = PlatformVersion::first();

            let result =
                drive.get_storage_fees_from_distribution_pool(Some(&transaction), platform_version);

            assert!(matches!(
                result,
                Err(Error::GroveDB(grovedb::Error::PathParentLayerNotFound(_)))
            ));
        }

        #[test]
        fn test_error_if_wrong_value_encoded() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let platform_version = PlatformVersion::first();

            let mut batch = GroveDbOpBatch::new();

            batch.add_insert(
                pools_vec_path(),
                KEY_STORAGE_FEE_POOL.to_vec(),
                Element::Item(u128::MAX.to_be_bytes().to_vec(), None),
            );

            drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let result =
                drive.get_storage_fees_from_distribution_pool(Some(&transaction), platform_version);

            assert!(matches!(
                result,
                Err(Error::Drive(DriveError::UnexpectedElementType(_))),
            ));
        }

        #[test]
        fn test_error_if_storage_pool_is_not_initiated() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let platform_version = PlatformVersion::first();

            // Remove storage pool key such as we would init the epoch
            // with `add_init_empty_without_storage_operations` method
            let mut batch = GroveDbOpBatch::new();

            batch.add_delete(pools_vec_path(), KEY_STORAGE_FEE_POOL.to_vec());

            drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let result = drive
                .get_storage_fees_from_distribution_pool(Some(&transaction), platform_version)
                .expect("should get aggregated storage fees");

            assert_eq!(result, 0);
        }
    }
}
