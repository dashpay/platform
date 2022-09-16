use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::fee_pools::constants;
use crate::platform::Platform;
use rs_drive::drive::batch::GroveDbOpBatch;
use rs_drive::drive::fee_pools::epochs::constants::{EPOCHS_PER_YEAR, PERPETUAL_STORAGE_YEARS};
use rs_drive::fee_pools::epochs::Epoch;
use rs_drive::grovedb::TransactionArg;
use rs_drive::{error, grovedb};
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;

pub type StorageDistributionLeftoverCredits = u64;

impl Platform {
    /// returns the leftovers
    pub fn add_distribute_storage_fee_distribution_pool_to_epochs_operations(
        &self,
        current_epoch_index: u16,
        transaction: TransactionArg,
        batch: &mut GroveDbOpBatch,
    ) -> Result<StorageDistributionLeftoverCredits, Error> {
        let storage_distribution_fees = self
            .drive
            .get_aggregate_storage_fees_from_distribution_pool(transaction)?;

        if storage_distribution_fees == 0 {
            return Ok(0);
        }

        // a separate buffer from which we withdraw to correctly calculate fee share
        let mut storage_distribution_leftover_credits = storage_distribution_fees;

        let storage_distribution_fees =
            Decimal::from_u64(storage_distribution_fees).ok_or(Error::Execution(
                ExecutionError::Overflow("storage distribution fees are not fitting in a u64"),
            ))?;

        let epochs_per_year = Decimal::from(EPOCHS_PER_YEAR);

        for year in 0..PERPETUAL_STORAGE_YEARS {
            let distribution_for_that_year_ratio = constants::FEE_DISTRIBUTION_TABLE[year as usize];

            let year_fee_share = storage_distribution_fees * distribution_for_that_year_ratio;

            let epoch_fee_share_dec = year_fee_share / epochs_per_year;

            let epoch_fee_share = epoch_fee_share_dec
                .floor()
                .to_u64()
                .ok_or(Error::Execution(ExecutionError::Overflow(
                    "storage distribution fees are not fitting in a u64",
                )))?;

            let year_start_epoch_index = current_epoch_index + EPOCHS_PER_YEAR * year;

            for index in year_start_epoch_index..year_start_epoch_index + EPOCHS_PER_YEAR {
                let epoch_tree = Epoch::new(index);

                let current_epoch_pool_storage_credits = self
                    .drive
                    .get_epoch_storage_credits_for_distribution(&epoch_tree, transaction)
                    .or_else(|e| match e {
                        // In case if we have a gap between current and previous epochs
                        // multiple future epochs could be created in the current batch
                        error::Error::GroveDB(grovedb::Error::PathNotFound(_))
                        | error::Error::GroveDB(grovedb::Error::PathKeyNotFound(_)) => Ok(0u64),
                        _ => Err(e),
                    })?;

                // TODO: It's not convenient and confusing when in once case you should push operation to batch
                //  and sometimes you pass batch inside to add operations. Also, in future a single operation function
                //  could become a multiple operations function so you need to change many code. Also, you can't use helpers which batch provides
                batch.push(
                    epoch_tree.update_storage_credits_for_distribution_operation(
                        current_epoch_pool_storage_credits + epoch_fee_share,
                    ),
                );

                storage_distribution_leftover_credits = storage_distribution_leftover_credits
                    .checked_sub(epoch_fee_share)
                    .ok_or(Error::Execution(ExecutionError::Overflow(
                        "leftover storage not fitting in a u64",
                    )))?;
            }
        }

        Ok(storage_distribution_leftover_credits)
    }
}

#[cfg(test)]
mod tests {

    mod distribute_storage_fee_distribution_pool {
        use crate::common::helpers::setup::setup_platform_with_initial_state_structure;
        use rs_drive::common::helpers::epoch::get_storage_credits_for_distribution_for_epochs_in_range;
        use rs_drive::drive::batch::GroveDbOpBatch;
        use rs_drive::error::drive::DriveError;
        use rs_drive::fee_pools::epochs::Epoch;
        use rs_drive::fee_pools::update_storage_fee_distribution_pool_operation;

        #[test]
        fn test_nothing_to_distribute() {
            let platform = setup_platform_with_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let epoch_index = 0;

            // Storage fee distribution pool is 0 after fee pools initialization

            let mut batch = GroveDbOpBatch::new();

            platform
                .add_distribute_storage_fee_distribution_pool_to_epochs_operations(
                    epoch_index,
                    Some(&transaction),
                    &mut batch,
                )
                .expect("should distribute storage fee pool");

            match platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
            {
                Ok(()) => assert!(false, "should return BatchIsEmpty error"),
                Err(e) => match e {
                    rs_drive::error::Error::Drive(DriveError::BatchIsEmpty()) => assert!(true),
                    _ => assert!(false, "invalid error type"),
                },
            }

            let storage_fees = get_storage_credits_for_distribution_for_epochs_in_range(
                &platform.drive,
                epoch_index..1000,
                Some(&transaction),
            );

            let reference_fees: Vec<u64> = (0..1000).map(|_| 0u64).collect();

            assert_eq!(storage_fees, reference_fees);
        }

        #[test]
        fn test_distribution_overflow() {
            let platform = setup_platform_with_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let storage_pool = u64::MAX;
            let epoch_index = 0;

            let mut batch = GroveDbOpBatch::new();

            batch.push(update_storage_fee_distribution_pool_operation(storage_pool));

            // Apply storage fee distribution pool update
            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let mut batch = GroveDbOpBatch::new();

            let leftovers = platform
                .add_distribute_storage_fee_distribution_pool_to_epochs_operations(
                    epoch_index,
                    Some(&transaction),
                    &mut batch,
                )
                .expect("should distribute storage fee pool");

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            // check leftover
            assert_eq!(leftovers, 515);
        }

        #[test]
        fn test_deterministic_distribution() {
            let platform = setup_platform_with_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let storage_pool = 1000000;
            let epoch_index = 42;

            let mut batch = GroveDbOpBatch::new();

            // init additional epochs pools as it will be done in epoch_change
            for i in 1000..=1000 + epoch_index {
                let epoch = Epoch::new(i);
                epoch.add_init_empty_operations(&mut batch);
            }

            batch.push(update_storage_fee_distribution_pool_operation(storage_pool));

            // Apply storage fee distribution pool update
            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let mut batch = GroveDbOpBatch::new();

            let leftovers = platform
                .add_distribute_storage_fee_distribution_pool_to_epochs_operations(
                    epoch_index,
                    Some(&transaction),
                    &mut batch,
                )
                .expect("should distribute storage fee pool");

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            // check leftover
            assert_eq!(leftovers, 180);

            // collect all the storage fee values of the 1000 epochs pools
            let storage_fees = get_storage_credits_for_distribution_for_epochs_in_range(
                &platform.drive,
                epoch_index..epoch_index + 1000,
                Some(&transaction),
            );

            // compare them with reference table
            #[rustfmt::skip]
            let reference_fees: [u64; 1000] = [
                2500, 2500, 2500, 2500, 2500, 2500, 2500, 2500, 2500, 2500, 2500, 2500, 2500, 2500,
                2500, 2500, 2500, 2500, 2500, 2500, 2400, 2400, 2400, 2400, 2400, 2400, 2400, 2400,
                2400, 2400, 2400, 2400, 2400, 2400, 2400, 2400, 2400, 2400, 2400, 2400, 2300, 2300,
                2300, 2300, 2300, 2300, 2300, 2300, 2300, 2300, 2300, 2300, 2300, 2300, 2300, 2300,
                2300, 2300, 2300, 2300, 2200, 2200, 2200, 2200, 2200, 2200, 2200, 2200, 2200, 2200,
                2200, 2200, 2200, 2200, 2200, 2200, 2200, 2200, 2200, 2200, 2100, 2100, 2100, 2100,
                2100, 2100, 2100, 2100, 2100, 2100, 2100, 2100, 2100, 2100, 2100, 2100, 2100, 2100,
                2100, 2100, 2000, 2000, 2000, 2000, 2000, 2000, 2000, 2000, 2000, 2000, 2000, 2000,
                2000, 2000, 2000, 2000, 2000, 2000, 2000, 2000, 1925, 1925, 1925, 1925, 1925, 1925,
                1925, 1925, 1925, 1925, 1925, 1925, 1925, 1925, 1925, 1925, 1925, 1925, 1925, 1925,
                1850, 1850, 1850, 1850, 1850, 1850, 1850, 1850, 1850, 1850, 1850, 1850, 1850, 1850,
                1850, 1850, 1850, 1850, 1850, 1850, 1775, 1775, 1775, 1775, 1775, 1775, 1775, 1775,
                1775, 1775, 1775, 1775, 1775, 1775, 1775, 1775, 1775, 1775, 1775, 1775, 1700, 1700,
                1700, 1700, 1700, 1700, 1700, 1700, 1700, 1700, 1700, 1700, 1700, 1700, 1700, 1700,
                1700, 1700, 1700, 1700, 1625, 1625, 1625, 1625, 1625, 1625, 1625, 1625, 1625, 1625,
                1625, 1625, 1625, 1625, 1625, 1625, 1625, 1625, 1625, 1625, 1550, 1550, 1550, 1550,
                1550, 1550, 1550, 1550, 1550, 1550, 1550, 1550, 1550, 1550, 1550, 1550, 1550, 1550,
                1550, 1550, 1475, 1475, 1475, 1475, 1475, 1475, 1475, 1475, 1475, 1475, 1475, 1475,
                1475, 1475, 1475, 1475, 1475, 1475, 1475, 1475, 1425, 1425, 1425, 1425, 1425, 1425,
                1425, 1425, 1425, 1425, 1425, 1425, 1425, 1425, 1425, 1425, 1425, 1425, 1425, 1425,
                1375, 1375, 1375, 1375, 1375, 1375, 1375, 1375, 1375, 1375, 1375, 1375, 1375, 1375,
                1375, 1375, 1375, 1375, 1375, 1375, 1325, 1325, 1325, 1325, 1325, 1325, 1325, 1325,
                1325, 1325, 1325, 1325, 1325, 1325, 1325, 1325, 1325, 1325, 1325, 1325, 1275, 1275,
                1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275,
                1275, 1275, 1275, 1275, 1225, 1225, 1225, 1225, 1225, 1225, 1225, 1225, 1225, 1225,
                1225, 1225, 1225, 1225, 1225, 1225, 1225, 1225, 1225, 1225, 1175, 1175, 1175, 1175,
                1175, 1175, 1175, 1175, 1175, 1175, 1175, 1175, 1175, 1175, 1175, 1175, 1175, 1175,
                1175, 1175, 1125, 1125, 1125, 1125, 1125, 1125, 1125, 1125, 1125, 1125, 1125, 1125,
                1125, 1125, 1125, 1125, 1125, 1125, 1125, 1125, 1075, 1075, 1075, 1075, 1075, 1075,
                1075, 1075, 1075, 1075, 1075, 1075, 1075, 1075, 1075, 1075, 1075, 1075, 1075, 1075,
                1025, 1025, 1025, 1025, 1025, 1025, 1025, 1025, 1025, 1025, 1025, 1025, 1025, 1025,
                1025, 1025, 1025, 1025, 1025, 1025, 975, 975, 975, 975, 975, 975, 975, 975, 975,
                975, 975, 975, 975, 975, 975, 975, 975, 975, 975, 975, 937, 937, 937, 937, 937,
                937, 937, 937, 937, 937, 937, 937, 937, 937, 937, 937, 937, 937, 937, 937, 900,
                900, 900, 900, 900, 900, 900, 900, 900, 900, 900, 900, 900, 900, 900, 900, 900,
                900, 900, 900, 862, 862, 862, 862, 862, 862, 862, 862, 862, 862, 862, 862, 862,
                862, 862, 862, 862, 862, 862, 862, 825, 825, 825, 825, 825, 825, 825, 825, 825,
                825, 825, 825, 825, 825, 825, 825, 825, 825, 825, 825, 787, 787, 787, 787, 787,
                787, 787, 787, 787, 787, 787, 787, 787, 787, 787, 787, 787, 787, 787, 787, 750,
                750, 750, 750, 750, 750, 750, 750, 750, 750, 750, 750, 750, 750, 750, 750, 750,
                750, 750, 750, 712, 712, 712, 712, 712, 712, 712, 712, 712, 712, 712, 712, 712,
                712, 712, 712, 712, 712, 712, 712, 675, 675, 675, 675, 675, 675, 675, 675, 675,
                675, 675, 675, 675, 675, 675, 675, 675, 675, 675, 675, 637, 637, 637, 637, 637,
                637, 637, 637, 637, 637, 637, 637, 637, 637, 637, 637, 637, 637, 637, 637, 600,
                600, 600, 600, 600, 600, 600, 600, 600, 600, 600, 600, 600, 600, 600, 600, 600,
                600, 600, 600, 562, 562, 562, 562, 562, 562, 562, 562, 562, 562, 562, 562, 562,
                562, 562, 562, 562, 562, 562, 562, 525, 525, 525, 525, 525, 525, 525, 525, 525,
                525, 525, 525, 525, 525, 525, 525, 525, 525, 525, 525, 487, 487, 487, 487, 487,
                487, 487, 487, 487, 487, 487, 487, 487, 487, 487, 487, 487, 487, 487, 487, 450,
                450, 450, 450, 450, 450, 450, 450, 450, 450, 450, 450, 450, 450, 450, 450, 450,
                450, 450, 450, 412, 412, 412, 412, 412, 412, 412, 412, 412, 412, 412, 412, 412,
                412, 412, 412, 412, 412, 412, 412, 375, 375, 375, 375, 375, 375, 375, 375, 375,
                375, 375, 375, 375, 375, 375, 375, 375, 375, 375, 375, 337, 337, 337, 337, 337,
                337, 337, 337, 337, 337, 337, 337, 337, 337, 337, 337, 337, 337, 337, 337, 300,
                300, 300, 300, 300, 300, 300, 300, 300, 300, 300, 300, 300, 300, 300, 300, 300,
                300, 300, 300, 262, 262, 262, 262, 262, 262, 262, 262, 262, 262, 262, 262, 262,
                262, 262, 262, 262, 262, 262, 262, 237, 237, 237, 237, 237, 237, 237, 237, 237,
                237, 237, 237, 237, 237, 237, 237, 237, 237, 237, 237, 212, 212, 212, 212, 212,
                212, 212, 212, 212, 212, 212, 212, 212, 212, 212, 212, 212, 212, 212, 212, 187,
                187, 187, 187, 187, 187, 187, 187, 187, 187, 187, 187, 187, 187, 187, 187, 187,
                187, 187, 187, 162, 162, 162, 162, 162, 162, 162, 162, 162, 162, 162, 162, 162,
                162, 162, 162, 162, 162, 162, 162, 137, 137, 137, 137, 137, 137, 137, 137, 137,
                137, 137, 137, 137, 137, 137, 137, 137, 137, 137, 137, 112, 112, 112, 112, 112,
                112, 112, 112, 112, 112, 112, 112, 112, 112, 112, 112, 112, 112, 112, 112, 87,
                87, 87, 87, 87, 87, 87, 87, 87, 87, 87, 87, 87, 87, 87, 87, 87, 87, 87, 87, 62,
                62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62
            ];

            assert_eq!(storage_fees, reference_fees);

            let total_distributed: u64 = storage_fees.iter().sum();

            assert_eq!(total_distributed + leftovers, storage_pool);

            /*

            Repeat distribution to ensure deterministic results

             */

            let mut batch = GroveDbOpBatch::new();

            // refill storage fee pool once more
            batch.push(update_storage_fee_distribution_pool_operation(storage_pool));

            // Apply storage fee distribution pool update
            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let mut batch = GroveDbOpBatch::new();

            // distribute fees once more
            platform
                .add_distribute_storage_fee_distribution_pool_to_epochs_operations(
                    epoch_index,
                    Some(&transaction),
                    &mut batch,
                )
                .expect("should distribute storage fee pool");

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            // collect all the storage fee values of the 1000 epochs pools again
            let storage_fees = get_storage_credits_for_distribution_for_epochs_in_range(
                &platform.drive,
                epoch_index..epoch_index + 1000,
                Some(&transaction),
            );

            // assert that all the values doubled meaning that distribution is reproducible
            assert_eq!(
                storage_fees,
                reference_fees
                    .iter()
                    .map(|val| val * 2)
                    .collect::<Vec<u64>>()
            );
        }
    }

    mod update_storage_fee_distribution_pool {
        use crate::common::helpers::setup::{
            setup_platform, setup_platform_with_initial_state_structure,
        };
        use rs_drive::drive::batch::GroveDbOpBatch;
        use rs_drive::error::Error as DriveError;
        use rs_drive::fee_pools::update_storage_fee_distribution_pool_operation;
        use rs_drive::grovedb;

        #[test]
        fn test_error_if_pool_is_not_initiated() {
            let platform = setup_platform();
            let transaction = platform.drive.grove.start_transaction();

            let storage_fee = 42;

            let mut batch = GroveDbOpBatch::new();

            batch.push(update_storage_fee_distribution_pool_operation(storage_fee));

            match platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
            {
                Ok(_) => assert!(
                    false,
                    "should not be able to update genesis time on uninit fee pools"
                ),
                Err(e) => match e {
                    DriveError::GroveDB(grovedb::Error::PathKeyNotFound(_)) => {
                        assert!(true)
                    }
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_update_and_get_value() {
            let platform = setup_platform_with_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let storage_fee = 42;

            let mut batch = GroveDbOpBatch::new();

            batch.push(update_storage_fee_distribution_pool_operation(storage_fee));

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let stored_storage_fee = platform
                .drive
                .get_aggregate_storage_fees_from_distribution_pool(Some(&transaction))
                .expect("should get storage fee pool");

            assert_eq!(storage_fee, stored_storage_fee);
        }
    }
}
