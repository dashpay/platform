mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::epoch::EpochIndex;
use dpp::fee::fee_result::LifetimeStorageFees;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    /// Reads the lifetime storage fee pools: the storage fees, by the epoch they were collected
    /// in and then by the number of epochs their storage lives, waiting for the next epoch
    /// change to spread them over those epochs (protocol version 14, document time to live).
    ///
    /// # Parameters
    /// - `transaction`: the transaction to read in.
    /// - `platform_version`: selects the method version.
    ///
    /// # Returns
    /// The credits of each lifetime pool, by its epoch and its number of epochs. A missing
    /// pools tree or a negative pool is corrupted state, and an error.
    pub fn fetch_lifetime_storage_fee_pools(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<EpochIndex, LifetimeStorageFees>, Error> {
        match platform_version
            .drive
            .methods
            .credit_pools
            .storage_fee_distribution_pool
            .fetch_lifetime_storage_fee_pools
        {
            0 => self.fetch_lifetime_storage_fee_pools_v0(transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_lifetime_storage_fee_pools".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::credit_pools::operations::update_lifetime_storage_fee_pool_operation;
    use crate::drive::credit_pools::paths::{
        lifetime_storage_fee_pool_key, lifetime_storage_fee_pools_vec_path,
    };
    use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use crate::util::batch::GroveDbOpBatch;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use grovedb::batch::QualifiedGroveDbOp;
    use grovedb::Element;

    #[test]
    fn should_read_the_pools_by_epoch_and_refuse_a_missing_tree_or_a_negative_pool() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let mut batch = GroveDbOpBatch::new();
        for (epoch_index, lifetime_epochs, credits) in [(3, 2, 7), (3, 40, 9), (4, 1, 5)] {
            batch.push(
                update_lifetime_storage_fee_pool_operation(epoch_index, lifetime_epochs, credits)
                    .expect("expected the pool operation"),
            );
        }
        drive
            .grove_apply_batch(batch, false, None, &platform_version.drive)
            .expect("expected to fill the pools");
        assert_eq!(
            drive
                .fetch_lifetime_storage_fee_pools(None, platform_version)
                .expect("expected to read the pools"),
            BTreeMap::from([
                (3, LifetimeStorageFees::from([(2, 7), (40, 9)])),
                (4, LifetimeStorageFees::from([(1, 5)])),
            ])
        );

        let mut batch = GroveDbOpBatch::new();
        batch.push(QualifiedGroveDbOp::insert_or_replace_op(
            lifetime_storage_fee_pools_vec_path(),
            lifetime_storage_fee_pool_key(4, 1),
            Element::new_sum_item(-5),
        ));
        drive
            .grove_apply_batch(batch, false, None, &platform_version.drive)
            .expect("expected to corrupt a pool");
        assert!(matches!(
            drive.fetch_lifetime_storage_fee_pools(None, platform_version),
            Err(Error::Drive(DriveError::CorruptedDriveState(_)))
        ));

        // A chain of protocol version 13 has no pools tree.
        let drive = setup_drive_with_initial_state_structure(Some(
            PlatformVersion::get(13).expect("expected protocol version 13"),
        ));
        assert!(matches!(
            drive.fetch_lifetime_storage_fee_pools(None, platform_version),
            Err(Error::Drive(DriveError::CorruptedDriveState(_)))
        ));
    }
}
