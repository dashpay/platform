use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::unpaid_epoch;
use crate::platform_types::platform::Platform;
use dpp::block::epoch::Epoch;
use dpp::fee::epoch::GENESIS_EPOCH_INDEX;
use dpp::version::PlatformVersion;
use drive::drive::credit_pools::epochs::start_block::StartBlockInfo;

use drive::grovedb::TransactionArg;

impl<C> Platform<C> {
    /// Finds and returns the oldest epoch that hasn't been paid out yet.
    /// The unpaid epoch potentially returned is always version 0
    pub(super) fn find_oldest_epoch_needing_payment_v0(
        &self,
        current_epoch_index: u16,
        cached_current_epoch_start_block_height: Option<u64>,
        cached_current_epoch_start_block_core_height: Option<u32>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<unpaid_epoch::v0::UnpaidEpochV0>, Error> {
        // Since we are paying for passed epochs there is nothing to do on genesis epoch
        if current_epoch_index == GENESIS_EPOCH_INDEX {
            return Ok(None);
        }

        let unpaid_epoch_index = self
            .drive
            .get_unpaid_epoch_index(transaction, platform_version)?;

        // We pay for previous epochs only
        if unpaid_epoch_index == current_epoch_index {
            return Ok(None);
        }

        let unpaid_epoch = Epoch::new(unpaid_epoch_index)?;

        let start_block_height = self.drive.get_epoch_start_block_height(
            &unpaid_epoch,
            transaction,
            platform_version,
        )?;

        let start_block_core_height = self.drive.get_epoch_start_block_core_height(
            &unpaid_epoch,
            transaction,
            platform_version,
        )?;

        let next_unpaid_epoch_info = if unpaid_epoch.index == current_epoch_index - 1 {
            // Use cached or committed block height for previous epoch
            let start_block_height = match cached_current_epoch_start_block_height {
                Some(start_block_height) => start_block_height,
                None => {
                    let current_epoch = Epoch::new(current_epoch_index)?;
                    self.drive.get_epoch_start_block_height(
                        &current_epoch,
                        transaction,
                        platform_version,
                    )?
                }
            };

            let start_block_core_height = match cached_current_epoch_start_block_core_height {
                Some(start_block_core_height) => start_block_core_height,
                None => {
                    let current_epoch = Epoch::new(current_epoch_index)?;
                    self.drive.get_epoch_start_block_core_height(
                        &current_epoch,
                        transaction,
                        platform_version,
                    )?
                }
            };
            StartBlockInfo {
                epoch_index: current_epoch_index,
                start_block_height,
                start_block_core_height,
            }
        } else {
            // Find a next epoch with start block height if unpaid epoch was more than one epoch ago
            match self.drive.get_first_epoch_start_block_info_between_epochs(
                unpaid_epoch.index,
                current_epoch_index,
                transaction,
                platform_version,
            )? {
                // Only possible on epoch change of current epoch, when we have start_block_height batched but not committed yet
                None => {
                    let Some(cached_current_epoch_start_block_height) =
                        cached_current_epoch_start_block_height
                    else {
                        return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "start_block_height must be present in current epoch or cached_next_epoch_start_block_height must be passed",
                        )));
                    };
                    let Some(cached_current_epoch_start_block_core_height) =
                        cached_current_epoch_start_block_core_height
                    else {
                        return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "start_block_core_height must be present in current epoch or cached_next_epoch_start_block_core_height must be passed",
                        )));
                    };
                    StartBlockInfo {
                        epoch_index: current_epoch_index,
                        start_block_height: cached_current_epoch_start_block_height,
                        start_block_core_height: cached_current_epoch_start_block_core_height,
                    }
                }
                Some(next_start_block_info) => next_start_block_info,
            }
        };

        // Use cached current epoch start block height only if we pay for the previous epoch

        Ok(Some(unpaid_epoch::v0::UnpaidEpochV0 {
            epoch_index: unpaid_epoch_index,
            next_unpaid_epoch_index: next_unpaid_epoch_info.epoch_index,
            start_block_height,
            next_epoch_start_block_height: next_unpaid_epoch_info.start_block_height,
            start_block_core_height,
            next_epoch_start_block_core_height: next_unpaid_epoch_info.start_block_core_height,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod find_oldest_epoch_needing_payment {
        use crate::execution::types::unpaid_epoch::v0::UnpaidEpochV0Methods;
        use crate::test::helpers::setup::TestPlatformBuilder;
        use drive::drive::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
        use drive::drive::batch::GroveDbOpBatch;
        use drive::fee_pools::epochs::operations_factory::EpochOperations;
        use drive::fee_pools::update_unpaid_epoch_index_operation;

        use super::*;

        #[test]
        fn test_no_epoch_to_pay_on_genesis_epoch() {
            let platform_version = PlatformVersion::latest();
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let unpaid_epoch = platform
                .find_oldest_epoch_needing_payment_v0(
                    GENESIS_EPOCH_INDEX,
                    None,
                    None,
                    Some(&transaction),
                    platform_version,
                )
                .expect("should find nothing");

            assert!(unpaid_epoch.is_none());
        }

        #[test]
        fn test_no_epoch_to_pay_if_oldest_unpaid_epoch_is_current_epoch() {
            let platform_version = PlatformVersion::latest();
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let epoch_0_tree = Epoch::new(GENESIS_EPOCH_INDEX).unwrap();

            let current_epoch_index = GENESIS_EPOCH_INDEX + 1;

            let epoch_1_tree = Epoch::new(current_epoch_index).unwrap();

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch_0_tree.update_start_block_height_operation(1));
            batch.push(epoch_1_tree.update_start_block_height_operation(2));

            batch.push(update_unpaid_epoch_index_operation(1));

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let unpaid_epoch = platform
                .find_oldest_epoch_needing_payment_v0(
                    current_epoch_index,
                    None,
                    None,
                    Some(&transaction),
                    platform_version,
                )
                .expect("should find nothing");

            assert!(unpaid_epoch.is_none());
        }

        #[test]
        fn test_use_cached_current_start_block_height_as_end_block_if_unpaid_epoch_is_previous() {
            let platform_version = PlatformVersion::latest();
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let epoch_0_tree = Epoch::new(GENESIS_EPOCH_INDEX).unwrap();

            let current_epoch_index = GENESIS_EPOCH_INDEX + 1;

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch_0_tree.update_start_block_height_operation(1));
            batch.push(epoch_0_tree.update_start_block_core_height_operation(1));

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let cached_current_epoch_start_block_height = Some(2);

            let cached_current_epoch_start_block_core_height = Some(2);

            let unpaid_epoch = platform
                .find_oldest_epoch_needing_payment_v0(
                    current_epoch_index,
                    cached_current_epoch_start_block_height,
                    cached_current_epoch_start_block_core_height,
                    Some(&transaction),
                    platform_version,
                )
                .expect("should find nothing");

            match unpaid_epoch {
                Some(unpaid_epoch) => {
                    assert_eq!(unpaid_epoch.epoch_index, 0);
                    assert_eq!(unpaid_epoch.next_unpaid_epoch_index, 1);
                    assert_eq!(unpaid_epoch.start_block_height, 1);
                    assert_eq!(unpaid_epoch.next_epoch_start_block_height, 2);

                    let block_count = unpaid_epoch
                        .block_count()
                        .expect("should calculate block count");

                    assert_eq!(block_count, 1);
                }
                None => unreachable!("unpaid epoch should be present"),
            }
        }

        #[test]
        fn test_use_stored_start_block_height_from_current_epoch_as_end_block_if_unpaid_epoch_is_previous(
        ) {
            let platform_version = PlatformVersion::latest();
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let epoch_0_tree = Epoch::new(GENESIS_EPOCH_INDEX).unwrap();

            let current_epoch_index = GENESIS_EPOCH_INDEX + 1;

            let epoch_1_tree = Epoch::new(current_epoch_index).unwrap();

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch_0_tree.update_start_block_height_operation(1));
            batch.push(epoch_0_tree.update_start_block_core_height_operation(1));
            batch.push(epoch_1_tree.update_start_block_height_operation(2));
            batch.push(epoch_1_tree.update_start_block_core_height_operation(2));

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let unpaid_epoch = platform
                .find_oldest_epoch_needing_payment_v0(
                    current_epoch_index,
                    None,
                    None,
                    Some(&transaction),
                    platform_version,
                )
                .expect("should find nothing");

            match unpaid_epoch {
                Some(unpaid_epoch) => {
                    assert_eq!(unpaid_epoch.epoch_index, 0);
                    assert_eq!(unpaid_epoch.next_unpaid_epoch_index, 1);
                    assert_eq!(unpaid_epoch.start_block_height, 1);
                    assert_eq!(unpaid_epoch.next_epoch_start_block_height, 2);

                    let block_count = unpaid_epoch
                        .block_count()
                        .expect("should calculate block count");

                    assert_eq!(block_count, 1);
                }
                None => unreachable!("unpaid epoch should be present"),
            }
        }

        #[test]
        fn test_find_stored_next_start_block_as_end_block_if_unpaid_epoch_more_than_one_ago() {
            let platform_version = PlatformVersion::latest();
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let epoch_0_tree = Epoch::new(GENESIS_EPOCH_INDEX).unwrap();
            let epoch_1_tree = Epoch::new(GENESIS_EPOCH_INDEX + 1).unwrap();

            let current_epoch_index = GENESIS_EPOCH_INDEX + 2;

            let epoch_2_tree = Epoch::new(current_epoch_index).unwrap();

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch_0_tree.update_start_block_height_operation(1));
            batch.push(epoch_0_tree.update_start_block_core_height_operation(1));
            batch.push(epoch_1_tree.update_start_block_height_operation(2));
            batch.push(epoch_1_tree.update_start_block_core_height_operation(2));
            batch.push(epoch_2_tree.update_start_block_height_operation(3));
            batch.push(epoch_2_tree.update_start_block_core_height_operation(3));

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let unpaid_epoch = platform
                .find_oldest_epoch_needing_payment_v0(
                    current_epoch_index,
                    None,
                    None,
                    Some(&transaction),
                    platform_version,
                )
                .expect("should find nothing");

            match unpaid_epoch {
                Some(unpaid_epoch) => {
                    assert_eq!(unpaid_epoch.epoch_index, 0);
                    assert_eq!(unpaid_epoch.next_unpaid_epoch_index, 1);
                    assert_eq!(unpaid_epoch.start_block_height, 1);
                    assert_eq!(unpaid_epoch.next_epoch_start_block_height, 2);

                    let block_count = unpaid_epoch
                        .block_count()
                        .expect("should calculate block count");

                    assert_eq!(block_count, 1);
                }
                None => unreachable!("unpaid epoch should be present"),
            }
        }

        #[test]
        fn test_use_cached_start_block_height_if_not_found_in_case_of_epoch_change() {
            let platform_version = PlatformVersion::latest();
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let epoch_0_tree = Epoch::new(GENESIS_EPOCH_INDEX).unwrap();

            let current_epoch_index = GENESIS_EPOCH_INDEX + 2;

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch_0_tree.update_start_block_height_operation(1));
            batch.push(epoch_0_tree.update_start_block_core_height_operation(1));

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let cached_current_epoch_start_block_height = Some(2);
            let cached_current_epoch_start_block_core_height = Some(2);

            let unpaid_epoch = platform
                .find_oldest_epoch_needing_payment_v0(
                    current_epoch_index,
                    cached_current_epoch_start_block_height,
                    cached_current_epoch_start_block_core_height,
                    Some(&transaction),
                    platform_version,
                )
                .expect("should find nothing");

            match unpaid_epoch {
                Some(unpaid_epoch) => {
                    assert_eq!(unpaid_epoch.epoch_index, 0);
                    assert_eq!(unpaid_epoch.next_unpaid_epoch_index, 2);
                    assert_eq!(unpaid_epoch.start_block_height, 1);
                    assert_eq!(unpaid_epoch.next_epoch_start_block_height, 2);

                    let block_count = unpaid_epoch
                        .block_count()
                        .expect("should calculate block count");

                    assert_eq!(block_count, 1);
                }
                None => unreachable!("unpaid epoch should be present"),
            }
        }

        #[test]
        fn test_error_if_cached_start_block_height_is_not_present_and_not_found_in_case_of_epoch_change(
        ) {
            let platform_version = PlatformVersion::latest();
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let epoch_0_tree = Epoch::new(GENESIS_EPOCH_INDEX).unwrap();

            let current_epoch_index = GENESIS_EPOCH_INDEX + 2;

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch_0_tree.update_start_block_height_operation(1));
            batch.push(epoch_0_tree.update_start_block_core_height_operation(1));

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let unpaid_epoch = platform.find_oldest_epoch_needing_payment_v0(
                current_epoch_index,
                None,
                None,
                Some(&transaction),
                platform_version,
            );

            assert!(matches!(
                unpaid_epoch,
                Err(Error::Execution(ExecutionError::CorruptedCodeExecution(_)))
            ));
        }
    }
}
