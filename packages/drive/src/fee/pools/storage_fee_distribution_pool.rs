use crate::drive::object_size_info::PathKeyElementInfo;
use crate::drive::storage::batch::Batch;
use crate::drive::Drive;
use grovedb::{Element, TransactionArg};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee::pools::epoch::epoch_pool::EpochPool;
use crate::fee::pools::fee_pools::FeePools;

use super::constants;

impl FeePools {
    pub fn distribute_storage_fee_distribution_pool(
        &self,
        drive: &Drive,
        batch: &mut Batch,
        epoch_index: u16,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        let storage_distribution_fees =
            self.get_storage_fee_distribution_pool_fees(drive, transaction)?;
        let storage_distribution_fees = Decimal::new(storage_distribution_fees, 0);

        // a separate buffer from which we withdraw to correctly calculate fee share
        let mut storage_distribution_fees_buffer = storage_distribution_fees;

        if storage_distribution_fees == dec!(0.0) {
            return Ok(());
        }

        for year in 0..50u16 {
            let distribution_percent = constants::FEE_DISTRIBUTION_TABLE[year as usize];

            let year_fee_share = storage_distribution_fees * distribution_percent;
            let epoch_fee_share = year_fee_share / dec!(20.0);

            let starting_epoch_index = epoch_index + year * 20;

            for index in starting_epoch_index..starting_epoch_index + 20 {
                let epoch_pool = EpochPool::new(index, drive);

                let storage_fee = epoch_pool.get_storage_fee(transaction)?;

                epoch_pool
                    .add_update_storage_fee_operations(batch, storage_fee + epoch_fee_share)?;

                storage_distribution_fees_buffer -= epoch_fee_share;
            }
        }

        let storage_distribution_fees_buffer =
            storage_distribution_fees_buffer.try_into().map_err(|_| {
                Error::Fee(FeeError::CorruptedStorageFeePoolInvalidItemLength(
                    "fee pools storage fee pool is not i64",
                ))
            })?;

        self.add_update_storage_fee_distribution_pool_operations(
            batch,
            storage_distribution_fees_buffer,
        )?;

        Ok(())
    }

    pub fn add_update_storage_fee_distribution_pool_operations(
        &self,
        batch: &mut Batch,
        storage_fee: i64,
    ) -> Result<(), Error> {
        batch.insert(PathKeyElementInfo::PathFixedSizeKeyElement((
            FeePools::get_path(),
            constants::KEY_STORAGE_FEE_POOL.as_slice(),
            Element::Item(storage_fee.to_le_bytes().to_vec(), None),
        )))
    }

    pub fn get_storage_fee_distribution_pool_fees(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
    ) -> Result<i64, Error> {
        let element = drive
            .grove
            .get(
                FeePools::get_path(),
                constants::KEY_STORAGE_FEE_POOL.as_slice(),
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        if let Element::Item(item, _) = element {
            let fee = i64::from_le_bytes(item.as_slice().try_into().map_err(|_| {
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
    use crate::drive::object_size_info::PathKeyElementInfo;
    use crate::fee::pools::tests::helpers::setup::{
        setup_drive, setup_fee_pools, SetupFeePoolsOptions,
    };
    use crate::{
        error::{self, fee::FeeError},
        fee::pools::{constants, fee_pools::FeePools},
    };
    use grovedb::Element;

    use crate::drive::storage::batch::Batch;

    mod helpers {
        use crate::drive::Drive;
        use crate::fee::pools::epoch::epoch_pool::EpochPool;
        use grovedb::TransactionArg;
        use rust_decimal::Decimal;

        pub fn get_storage_fees_from_epoch_pools(
            drive: &Drive,
            epoch_index: u16,
            transaction: TransactionArg,
        ) -> Vec<Decimal> {
            (epoch_index..epoch_index + 1000)
                .map(|index| {
                    let epoch_pool = EpochPool::new(index, &drive);
                    epoch_pool
                        .get_storage_fee(transaction)
                        .expect("should get storage fee")
                })
                .collect()
        }
    }

    mod distribute_storage_fee_distribution_pool {
        use crate::error::drive::DriveError;
        use crate::error::Error;
        use rust_decimal::Decimal;
        use rust_decimal_macros::dec;

        use crate::fee::pools::epoch::epoch_pool::EpochPool;
        use crate::fee::pools::storage_fee_distribution_pool::tests::helpers;
        use crate::fee::pools::tests::helpers::setup::{setup_drive, setup_fee_pools};

        #[test]
        fn test_nothing_to_distribute() {
            let drive = setup_drive();
            let (transaction, fee_pools) = setup_fee_pools(&drive, None);

            let epoch_index = 0;

            // Storage fee distribution pool is 0 after fee pools initialization

            let mut batch = super::Batch::new(&drive);

            fee_pools
                .distribute_storage_fee_distribution_pool(
                    &drive,
                    &mut batch,
                    epoch_index,
                    Some(&transaction),
                )
                .expect("should distribute storage fee pool");

            match drive.apply_batch(batch, false, Some(&transaction)) {
                Ok(()) => assert!(false, "should return BatchIsEmpty error"),
                Err(e) => match e {
                    Error::Drive(DriveError::BatchIsEmpty()) => assert!(true),
                    _ => assert!(false, "invalid error type"),
                },
            }

            let storage_fees =
                helpers::get_storage_fees_from_epoch_pools(&drive, epoch_index, Some(&transaction));

            let reference_fees: Vec<Decimal> = (0..1000).map(|_| dec!(0)).collect();

            assert_eq!(storage_fees, reference_fees);
        }

        #[test]
        fn test_distribution_overflow() {
            let drive = setup_drive();
            let (transaction, fee_pools) = setup_fee_pools(&drive, None);

            let storage_pool = i64::MAX;
            let epoch_index = 0;

            let mut batch = super::Batch::new(&drive);

            fee_pools
                .add_update_storage_fee_distribution_pool_operations(&mut batch, storage_pool)
                .expect("should update storage fee pool");

            // Apply storage fee distribution pool update
            drive
                .apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let mut batch = super::Batch::new(&drive);

            fee_pools
                .distribute_storage_fee_distribution_pool(
                    &drive,
                    &mut batch,
                    epoch_index,
                    Some(&transaction),
                )
                .expect("should distribute storage fee pool");

            drive
                .apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            // check leftover
            let storage_fee_pool_leftover = fee_pools
                .get_storage_fee_distribution_pool_fees(&drive, Some(&transaction))
                .expect("should get storage fee pool");

            assert_eq!(storage_fee_pool_leftover, 0);
        }

        #[test]
        fn test_deterministic_distribution() {
            let drive = setup_drive();
            let (transaction, fee_pools) = setup_fee_pools(&drive, None);

            let storage_pool = 1000;
            let epoch_index = 42;

            let mut batch = super::Batch::new(&drive);

            // init additional epoch pools as it will be done in epoch_change
            for i in 1000..=1000 + epoch_index {
                let epoch = EpochPool::new(i, &drive);
                epoch
                    .add_init_empty_operations(&mut batch)
                    .expect("should init additional epoch pool");
            }

            fee_pools
                .add_update_storage_fee_distribution_pool_operations(&mut batch, storage_pool)
                .expect("should update storage fee pool");

            // Apply storage fee distribution pool update
            drive
                .apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let mut batch = super::Batch::new(&drive);

            fee_pools
                .distribute_storage_fee_distribution_pool(
                    &drive,
                    &mut batch,
                    epoch_index,
                    Some(&transaction),
                )
                .expect("should distribute storage fee pool");

            drive
                .apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            // check leftover
            let storage_fee_pool_leftover = fee_pools
                .get_storage_fee_distribution_pool_fees(&drive, Some(&transaction))
                .expect("should get storage fee pool");

            assert_eq!(storage_fee_pool_leftover, 0);

            // collect all the storage fee values of the 1000 epoch pools
            let storage_fees =
                helpers::get_storage_fees_from_epoch_pools(&drive, epoch_index, Some(&transaction));

            // compare them with reference table
            #[rustfmt::skip]
            let reference_fees = [
                dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000),
                dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000),
                dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000),
                dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000),
                dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000),
                dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000),
                dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000),
                dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000),
                dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000),
                dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000),
                dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000),
                dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000),
                dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250),
                dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250),
                dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500),
                dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500),
                dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750),
                dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750),
                dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000),
                dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000),
                dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250),
                dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250),
                dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500),
                dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500),
                dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750),
                dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750),
                dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250),
                dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250),
                dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750),
                dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750),
                dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250),
                dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250),
                dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750),
                dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750),
                dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250),
                dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250),
                dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750),
                dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750),
                dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250),
                dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250),
                dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750),
                dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750),
                dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250),
                dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250),
                dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750),
                dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750),
                dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375),
                dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375),
                dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000),
                dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000),
                dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625),
                dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625),
                dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250),
                dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250),
                dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875),
                dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875),
                dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500),
                dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500),
                dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125),
                dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125),
                dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750),
                dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750),
                dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375),
                dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375),
                dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000),
                dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000),
                dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625),
                dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625),
                dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250),
                dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250),
                dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875),
                dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875),
                dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500),
                dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500),
                dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125),
                dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125),
                dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750),
                dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750),
                dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375),
                dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375),
                dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000),
                dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000),
                dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625),
                dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625),
                dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375),
                dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375),
                dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125),
                dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125),
                dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875),
                dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875),
                dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625),
                dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625),
                dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375),
                dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375),
                dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125),
                dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125),
                dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875),
                dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875),
                dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625),
                dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625),
            ];

            assert_eq!(storage_fees, reference_fees);

            /*

            Repeat distribution to ensure deterministic results

             */

            let mut batch = super::Batch::new(&drive);

            // refill storage fee pool once more
            fee_pools
                .add_update_storage_fee_distribution_pool_operations(&mut batch, storage_pool)
                .expect("should update storage fee pool");

            // Apply storage fee distribution pool update
            drive
                .apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let mut batch = super::Batch::new(&drive);

            // distribute fees once more
            fee_pools
                .distribute_storage_fee_distribution_pool(
                    &drive,
                    &mut batch,
                    epoch_index,
                    Some(&transaction),
                )
                .expect("should distribute storage fee pool");

            drive
                .apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            // collect all the storage fee values of the 1000 epoch pools again
            let storage_fees =
                helpers::get_storage_fees_from_epoch_pools(&drive, epoch_index, Some(&transaction));

            // assert that all the values doubled meaning that distribution is reproducible
            assert_eq!(
                storage_fees,
                reference_fees
                    .iter()
                    .map(|val| val * dec!(2))
                    .collect::<Vec<Decimal>>()
            );
        }
    }

    mod update_storage_fee_distribution_pool {
        #[test]
        fn test_error_if_pool_is_not_initiated() {
            let drive = super::setup_drive();
            let (transaction, fee_pools) = super::setup_fee_pools(
                &drive,
                Some(super::SetupFeePoolsOptions {
                    apply_fee_pool_structure: false,
                }),
            );

            let storage_fee = 42;

            let mut batch = super::Batch::new(&drive);

            fee_pools
                .add_update_storage_fee_distribution_pool_operations(&mut batch, storage_fee)
                .expect("should update storage fee distribution pool");

            match drive.apply_batch(batch, false, Some(&transaction)) {
                Ok(_) => assert!(
                    false,
                    "should not be able to update genesis time on uninit fee pools"
                ),
                Err(e) => match e {
                    super::error::Error::GroveDB(grovedb::Error::PathKeyNotFound(_)) => {
                        assert!(true)
                    }
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_update_and_get_value() {
            let drive = super::setup_drive();
            let (transaction, fee_pools) = super::setup_fee_pools(&drive, None);

            let storage_fee = 42;

            let mut batch = super::Batch::new(&drive);

            fee_pools
                .add_update_storage_fee_distribution_pool_operations(&mut batch, storage_fee)
                .expect("should update storage fee pool");

            drive
                .apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let stored_storage_fee = fee_pools
                .get_storage_fee_distribution_pool_fees(&drive, Some(&transaction))
                .expect("should get storage fee pool");

            assert_eq!(storage_fee, stored_storage_fee);
        }
    }

    mod get_storage_fee_distribution_pool_fees {
        #[test]
        fn test_error_if_pool_is_not_initiated() {
            let drive = super::setup_drive();
            let (transaction, fee_pools) = super::setup_fee_pools(
                &drive,
                Some(super::SetupFeePoolsOptions {
                    apply_fee_pool_structure: false,
                }),
            );

            match fee_pools.get_storage_fee_distribution_pool_fees(&drive, Some(&transaction)) {
                Ok(_) => assert!(
                    false,
                    "should not be able to get genesis time on uninit fee pools"
                ),
                Err(e) => match e {
                    super::error::Error::GroveDB(grovedb::Error::PathNotFound(_)) => assert!(true),
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_error_if_wrong_value_encoded() {
            let drive = super::setup_drive();
            let (transaction, fee_pools) = super::setup_fee_pools(&drive, None);

            let mut batch = super::Batch::new(&drive);

            batch
                .insert(super::PathKeyElementInfo::PathFixedSizeKeyElement((
                    super::FeePools::get_path(),
                    super::constants::KEY_STORAGE_FEE_POOL.as_slice(),
                    super::Element::Item(u128::MAX.to_le_bytes().to_vec(), None),
                )))
                .expect("should insert invalid data");

            drive
                .apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            match fee_pools.get_storage_fee_distribution_pool_fees(&drive, Some(&transaction)) {
                Ok(_) => assert!(false, "should not be able to decode stored value"),
                Err(e) => match e {
                    super::error::Error::Fee(
                        super::FeeError::CorruptedStorageFeePoolInvalidItemLength(_),
                    ) => {
                        assert!(true)
                    }
                    _ => assert!(false, "invalid error type"),
                },
            }
        }
    }
}
