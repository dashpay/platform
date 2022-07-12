use grovedb::{Element, TransactionArg};

use crate::drive::abci::messages::Fees;
use crate::drive::block::BlockInfo;
use crate::drive::object_size_info::{KeyInfo, PathKeyElementInfo};
use crate::drive::storage::batch::Batch;
use crate::drive::{Drive, RootTree};
use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee::epoch::EpochInfo;
use crate::fee::pools::fee_distribution::DistributionInfo;

use super::constants;
use super::epoch::epoch_pool::EpochPool;

pub struct FeePools {
    // TODO: Accept storage (which incapsulates grove_operations) to share it with all methods
    // TODO: Accept identities struct to use in specific methods
}

impl Default for FeePools {
    fn default() -> Self {
        Self::new()
    }
}

impl FeePools {
    pub fn new() -> FeePools {
        FeePools {}
    }

    pub fn get_path<'a>() -> [&'a [u8]; 1] {
        [Into::<&[u8; 1]>::into(RootTree::Pools)]
    }

    pub fn add_create_fee_pool_trees_operations(
        &self,
        drive: &Drive,
        batch: &mut Batch,
    ) -> Result<(), Error> {
        // init fee pool subtree
        batch.insert_empty_tree([], KeyInfo::KeyRef(FeePools::get_path()[0]), None)?;

        // Update storage credit pool
        batch.insert(PathKeyElementInfo::PathFixedSizeKeyElement((
            FeePools::get_path(),
            constants::KEY_STORAGE_FEE_POOL.as_slice(),
            Element::Item(0i64.to_le_bytes().to_vec(), None),
        )))?;

        // We need to insert 50 years worth of epochs,
        // with 20 epochs per year that's 1000 epochs
        for i in 0..1000 {
            let epoch = EpochPool::new(i, drive);
            epoch.add_init_empty_operations(batch)?;
        }

        Ok(())
    }

    pub fn add_shift_current_epoch_pool_operations(
        &self,
        drive: &Drive,
        batch: &mut Batch,
        current_epoch_pool: &EpochPool,
        start_block_height: u64,
        start_block_time: i64,
        fee_multiplier: u64,
    ) -> Result<(), Error> {
        // create and init next thousandth epoch
        let next_thousandth_epoch = EpochPool::new(current_epoch_pool.index + 1000, drive);
        next_thousandth_epoch.add_init_empty_operations(batch)?;

        // init first_proposer_block_height and processing_fee for an epoch
        current_epoch_pool.add_init_current_operations(
            batch,
            fee_multiplier,
            start_block_height,
            start_block_time,
        )?;

        Ok(())
    }

    pub fn process_block_fees(
        &self,
        drive: &Drive,
        block_info: &BlockInfo,
        epoch_info: &EpochInfo,
        fees: &Fees,
        transaction: TransactionArg,
    ) -> Result<DistributionInfo, Error> {
        let current_epoch_pool = EpochPool::new(epoch_info.current_epoch_index, drive);

        if epoch_info.is_epoch_change {
            let mut batch = Batch::new(drive);

            // make next epoch pool as a current
            // and create one more in future
            self.add_shift_current_epoch_pool_operations(
                drive,
                &mut batch,
                &current_epoch_pool,
                block_info.block_height,
                block_info.block_time,
                fees.fee_multiplier,
            )?;

            // distribute accumulated previous epoch storage fees
            if current_epoch_pool.index > 0 {
                self.distribute_storage_fee_distribution_pool(
                    drive,
                    &mut batch,
                    current_epoch_pool.index - 1,
                    transaction,
                )?;
            }

            // We need to apply new epoch tree structure and distributed storage fee
            drive.apply_batch(batch, false, transaction)?;
        }

        let mut batch = Batch::new(drive);

        current_epoch_pool.add_increment_proposer_block_count_operations(
            &mut batch,
            &block_info.proposer_pro_tx_hash,
            transaction,
        )?;

        let distribution_info = self
            .add_distribute_fees_from_unpaid_pools_to_proposers_operations(
                drive,
                &mut batch,
                epoch_info.current_epoch_index,
                transaction,
            )?;

        // Move integer part of the leftovers to processing
        // and fractional part to storage fees for the upcoming epoch
        let storage_fees_leftovers: i64 = (distribution_info.fee_leftovers.fract())
            .try_into()
            .map_err(|_| {
                Error::Fee(FeeError::DecimalConversion(
                    "can't convert storage fees leftovers from Decimal to i64",
                ))
            })?;
        let processing_fees_leftovers: u64 = (distribution_info.fee_leftovers.floor())
            .try_into()
            .map_err(|_| {
            Error::Fee(FeeError::DecimalConversion(
                "can't convert processing fees leftover from Decimal to u64",
            ))
        })?;

        self.add_distribute_fees_into_pools_operations(
            drive,
            &mut batch,
            &current_epoch_pool,
            fees.processing_fees + processing_fees_leftovers,
            fees.storage_fees + storage_fees_leftovers,
            transaction,
        )?;

        drive.apply_if_not_empty(batch, false, transaction)?;

        Ok(distribution_info)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        error,
        fee::pools::tests::helpers::setup::{setup_drive, setup_fee_pools},
    };

    use rust_decimal_macros::dec;

    use crate::drive::storage::batch::Batch;
    use crate::fee::pools::epoch::epoch_pool::EpochPool;

    mod create_fee_pool_trees {
        #[test]
        fn test_values_are_set() {
            let drive = super::setup_drive();
            let (transaction, fee_pools) = super::setup_fee_pools(&drive, None);

            let storage_fee_pool = fee_pools
                .get_storage_fee_distribution_pool_fees(&drive, Some(&transaction))
                .expect("should get storage fee pool");

            assert_eq!(storage_fee_pool, 0i64);
        }

        #[test]
        fn test_epoch_pools_are_created() {
            let drive = super::setup_drive();
            let (transaction, _) = super::setup_fee_pools(&drive, None);

            for epoch_index in 0..1000 {
                let epoch_pool = super::EpochPool::new(epoch_index, &drive);

                let storage_fee = epoch_pool
                    .get_storage_fee(Some(&transaction))
                    .expect("should get storage fee");

                assert_eq!(storage_fee, super::dec!(0));
            }

            let epoch_pool = super::EpochPool::new(1000, &drive); // 1001th epoch pool

            match epoch_pool.get_storage_fee(Some(&transaction)) {
                Ok(_) => assert!(false, "must be an error"),
                Err(e) => match e {
                    super::error::Error::GroveDB(_) => assert!(true),
                    _ => assert!(false, "invalid error type"),
                },
            }
        }
    }

    mod shift_current_epoch_pool {
        #[test]
        fn test_values_are_set() {
            let drive = super::setup_drive();
            let (transaction, fee_pools) = super::setup_fee_pools(&drive, None);

            let current_epoch_pool = super::EpochPool::new(0, &drive);

            let start_block_height = 10;
            let start_block_time = 1655396517912;
            let multiplier = 42;

            let mut batch = super::Batch::new(&drive);

            fee_pools
                .add_shift_current_epoch_pool_operations(
                    &drive,
                    &mut batch,
                    &current_epoch_pool,
                    start_block_height,
                    start_block_time,
                    multiplier,
                )
                .expect("should shift epoch pool");

            drive
                .apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let next_thousandth_epoch = super::EpochPool::new(1000, &drive);

            let storage_fee_pool = next_thousandth_epoch
                .get_storage_fee(Some(&transaction))
                .expect("should get storage fee");

            assert_eq!(storage_fee_pool, super::dec!(0));

            let stored_start_block_height = current_epoch_pool
                .get_start_block_height(Some(&transaction))
                .expect("should get start block height");

            assert_eq!(stored_start_block_height, start_block_height);

            let stored_start_block_time = current_epoch_pool
                .get_start_time(Some(&transaction))
                .expect("should get start time");

            assert_eq!(stored_start_block_time, start_block_time);

            let stored_multiplier = current_epoch_pool
                .get_fee_multiplier(Some(&transaction))
                .expect("should get fee multiplier");

            assert_eq!(stored_multiplier, multiplier);
        }
    }
}
