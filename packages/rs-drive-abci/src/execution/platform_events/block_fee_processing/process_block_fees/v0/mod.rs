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

//! Block Fees Processing.
//!
//! This modules defines functions related to processing block fees upon block and
//! epoch changes.
//!

use std::option::Option::None;

use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;
use drive::drive::batch::DriveOperation;
use drive::drive::Drive;
use drive::grovedb::Transaction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_fees::v0::BlockFeesV0Getters;
use crate::execution::types::block_fees::BlockFees;
use crate::execution::types::block_state_info::v0::{
    BlockStateInfoV0Getters, BlockStateInfoV0Methods,
};
use crate::execution::types::block_state_info::BlockStateInfo;
use crate::execution::types::processed_block_fees_outcome;
use crate::platform_types::epoch_info::v0::EpochInfoV0Getters;
use crate::platform_types::epoch_info::EpochInfo;
use crate::platform_types::platform::Platform;
use drive::fee_pools::epochs::operations_factory::EpochOperations;

/// From the Dash Improvement Proposal:

/// For the purpose of this explanation we can trivialize that the execution of a block comprises
/// the sum of the execution of all state transitions contained within the block. In order to
/// avoid altering participating masternode identity balances every block and distribute fees
/// evenly, the concept of pools is introduced. We will also introduce the concepts of an Epoch
/// and the Epoch Era that are both covered later in this document. As the block executes state
/// transitions, processing and storage fees are accumulated, as well as a list of refunded fees
/// from various Epochs and fee multipliers. When there are no more state transitions to execute
/// we can say the block has ended its state transition execution phase. The system will then add
/// the accumulated fees to their corresponding pools, and in the case of deletion of data, remove
/// storage fees from future Epoch storage pools.

impl<CoreRPCLike> Platform<CoreRPCLike> {
    /// Adds operations to GroveDB op batch related to processing
    /// and distributing the block fees from the previous block and applies the batch.
    ///
    /// Returns `ProcessedBlockFeesOutcome`.
    #[inline(always)]
    pub(super) fn process_block_fees_v0(
        &self,
        block_info: &BlockStateInfo,
        epoch_info: &EpochInfo,
        block_fees: BlockFees,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<processed_block_fees_outcome::v0::ProcessedBlockFeesOutcome, Error> {
        let current_epoch = Epoch::new(epoch_info.current_epoch_index())?;

        let mut batch = vec![];

        let storage_fee_distribution_outcome = if epoch_info.is_epoch_change() {
            self.add_process_epoch_change_operations(
                block_info,
                epoch_info,
                &block_fees,
                transaction,
                &mut batch,
                platform_version,
            )?
        } else {
            None
        };

        // Since epoch pool tree batched is not committed yet
        // we pass previous block count explicitly
        let cached_previous_block_count = if epoch_info.is_epoch_change() {
            Some(0)
        } else {
            None
        };

        batch.push(DriveOperation::GroveDBOperation(
            current_epoch.increment_proposer_block_count_operation(
                &self.drive,
                &block_info.proposer_pro_tx_hash(),
                cached_previous_block_count,
                Some(transaction),
                platform_version,
            )?,
        ));

        // Distribute fees from unpaid epoch pool to proposers

        // Since start_block_height for current epoch is batched and not committed yet
        // we pass it explicitly
        let (cached_current_epoch_start_block_height, cached_current_epoch_start_block_core_height) =
            if epoch_info.is_epoch_change() {
                (
                    Some(block_info.height()),
                    Some(block_info.core_chain_locked_height()),
                )
            } else {
                (None, None)
            };

        let payouts = if epoch_info.is_epoch_change() {
            self.add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations(
                epoch_info.current_epoch_index(),
                cached_current_epoch_start_block_height,
                cached_current_epoch_start_block_core_height,
                transaction,
                &mut batch,
                platform_version,
            )?
        } else {
            None
        };

        let fees_in_pools = self.add_distribute_block_fees_into_pools_operations(
            &current_epoch,
            &block_fees,
            // Add leftovers after storage fee pool distribution to the current block storage fees
            storage_fee_distribution_outcome
                .as_ref()
                .map(|outcome| outcome.leftovers),
            Some(transaction),
            &mut batch,
            platform_version,
        )?;

        let pending_epoch_refunds = if !epoch_info.is_epoch_change() {
            self.drive
                .fetch_and_add_pending_epoch_refunds_to_collection(
                    block_fees.refunds_per_epoch_owned(),
                    Some(transaction),
                    &platform_version.drive,
                )?
        } else {
            block_fees.refunds_per_epoch_owned()
        };

        Drive::add_update_pending_epoch_refunds_operations(
            &mut batch,
            pending_epoch_refunds,
            &platform_version.drive,
        )?;

        self.drive.apply_drive_operations(
            batch,
            true,
            &block_info.to_block_info(epoch_info.try_into()?),
            Some(transaction),
            platform_version,
        )?;

        let outcome = processed_block_fees_outcome::v0::ProcessedBlockFeesOutcome {
            fees_in_pools,
            payouts,
            refunded_epochs_count: storage_fee_distribution_outcome
                .map(|outcome| outcome.refunded_epochs_count),
        };

        if self.config.execution.verify_sum_trees {
            // Verify sum trees
            let credits_verified = self
                .drive
                .calculate_total_credits_balance(Some(transaction), &platform_version.drive)
                .map_err(Error::Drive)?;

            if !credits_verified.ok()? {
                return Err(Error::Execution(
                    ExecutionError::CorruptedCreditsNotBalanced(format!(
                        "credits are not balanced after block execution {:?} off by {}",
                        credits_verified,
                        credits_verified
                            .total_in_trees()
                            .unwrap()
                            .abs_diff(credits_verified.total_credits_in_platform)
                    )),
                ));
            }
        }

        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Utc;
    use dpp::fee::epoch::GENESIS_EPOCH_INDEX;
    use rust_decimal::prelude::ToPrimitive;

    use crate::config::ExecutionConfig;
    use crate::{config::PlatformConfig, test::helpers::setup::TestPlatformBuilder};
    use drive::common::identities::create_test_masternode_identities;

    mod helpers {
        use super::*;
        use crate::execution::types::block_fees::v0::BlockFeesV0;
        use crate::execution::types::block_state_info::v0::BlockStateInfoV0;
        use crate::platform_types::epoch_info::v0::EpochInfoV0;
        use dpp::fee::epoch::{perpetual_storage_epochs, CreditsPerEpoch, GENESIS_EPOCH_INDEX};

        /// Process and validate block fees
        pub fn process_and_validate_block_fees<C>(
            platform: &Platform<C>,
            genesis_time_ms: u64,
            epoch_index: u16,
            block_height: u64,
            previous_block_time_ms: Option<u64>,
            proposer_pro_tx_hash: [u8; 32],
            transaction: &Transaction,
        ) -> BlockStateInfoV0 {
            let current_epoch = Epoch::new(epoch_index).unwrap();
            let platform_version = PlatformVersion::latest();

            let block_time_ms = genesis_time_ms
                + epoch_index as u64 * platform.config.execution.epoch_time_length_s * 1000
                + block_height;

            let block_info = BlockStateInfoV0 {
                height: block_height,
                round: 0,
                block_time_ms,
                previous_block_time_ms,
                proposer_pro_tx_hash,
                core_chain_locked_height: 1,
                block_hash: None,
                app_hash: None,
            };

            let epoch_info = EpochInfoV0::from_genesis_time_and_block_info(
                genesis_time_ms,
                &block_info,
                platform.config.execution.epoch_time_length_s,
            )
            .expect("should calculate epoch info")
            .into();

            let block_fees: BlockFees = BlockFeesV0 {
                storage_fee: 100000,
                processing_fee: 10000,
                refunds_per_epoch: CreditsPerEpoch::from_iter([(epoch_index, 100)]),
            }
            .into();

            let storage_fee_distribution_outcome = platform
                .process_block_fees_v0(
                    &block_info.clone().into(),
                    &epoch_info,
                    block_fees.clone(),
                    transaction,
                    platform_version,
                )
                .expect("should process block fees");

            // Should process epoch change
            // and distribute aggregated storage fees into pools on epoch > 0

            let aggregated_storage_fees = platform
                .drive
                .get_storage_fees_from_distribution_pool(Some(transaction), platform_version)
                .expect("should get storage fees from distribution pool");

            if epoch_info.is_epoch_change() {
                if epoch_info.current_epoch_index() == GENESIS_EPOCH_INDEX {
                    assert_eq!(aggregated_storage_fees, block_fees.storage_fee());
                } else {
                    // Assuming leftovers
                    // we have perpetual_storage_epochs(platform.drive.config.epochs_per_era) as
                    // there could be 1 per epoch left over
                    assert!(
                        block_fees.storage_fee() <= aggregated_storage_fees
                            && aggregated_storage_fees
                                < block_fees.storage_fee()
                                    + perpetual_storage_epochs(platform.drive.config.epochs_per_era)
                                        as u64
                    );
                };
            } else {
                assert!(aggregated_storage_fees > block_fees.storage_fee());
            }

            // Should increment proposer block count

            let proposers_block_count = platform
                .drive
                .get_epochs_proposer_block_count(
                    &current_epoch,
                    &proposer_pro_tx_hash,
                    Some(transaction),
                    platform_version,
                )
                .expect("should get proposers");

            assert_ne!(proposers_block_count, 0);

            // Should pay for previous epoch

            if epoch_info.is_epoch_change() && epoch_index > GENESIS_EPOCH_INDEX {
                assert!(storage_fee_distribution_outcome.payouts.is_some());
            } else {
                assert!(storage_fee_distribution_outcome.payouts.is_none());
            }

            // Should distribute block fees into pools

            let processing_fees = platform
                .drive
                .get_epoch_processing_credits_for_distribution(
                    &current_epoch,
                    Some(transaction),
                    platform_version,
                )
                .expect("should get processing credits");

            assert_ne!(processing_fees, 0);

            block_info
        }
    }

    #[test]
    fn test_process_3_block_fees_from_different_epochs() {
        let platform_version = PlatformVersion::latest();
        // We are not adding to the overall platform credits so we can't verify
        // the sum trees
        let platform = TestPlatformBuilder::new()
            .with_config(PlatformConfig {
                execution: ExecutionConfig {
                    verify_sum_trees: false,
                    ..Default::default()
                },
                ..Default::default()
            })
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let transaction = platform.drive.grove.start_transaction();

        platform.create_mn_shares_contract(Some(&transaction), platform_version);

        let proposers = create_test_masternode_identities(
            &platform.drive,
            6,
            Some(56),
            Some(&transaction),
            platform_version,
        );

        let genesis_time_ms = Utc::now()
            .timestamp_millis()
            .to_u64()
            .expect("block time can not be before 1970");

        /*
        Process first block of epoch 0 (genesis epoch)

        Should change epoch to 0
        Should not pay to proposers
         */

        let epoch_index = GENESIS_EPOCH_INDEX;
        let mut block_height = 1;

        let block_info = helpers::process_and_validate_block_fees(
            &platform,
            genesis_time_ms,
            epoch_index,
            block_height,
            None,
            proposers[0],
            &transaction,
        );

        /*
        Process second block of epoch 0

        Should not change epoch
        Should not pay to proposers
         */

        let epoch_index = GENESIS_EPOCH_INDEX;
        block_height += 1;

        let block_info = helpers::process_and_validate_block_fees(
            &platform,
            genesis_time_ms,
            epoch_index,
            block_height,
            Some(block_info.block_time_ms),
            proposers[1],
            &transaction,
        );

        /*
        Process first block of epoch 1

        Should change epoch to 1
        Should pay to proposers from epoch 0
         */

        let epoch_index = GENESIS_EPOCH_INDEX + 1;
        block_height += 1;

        let block_info = helpers::process_and_validate_block_fees(
            &platform,
            genesis_time_ms,
            epoch_index,
            block_height,
            Some(block_info.block_time_ms),
            proposers[2],
            &transaction,
        );

        /*
        Process second block of epoch 1

        Should not change epoch
        Should not pay to proposers 0
         */

        let epoch_index = GENESIS_EPOCH_INDEX + 1;
        block_height += 1;

        let block_info = helpers::process_and_validate_block_fees(
            &platform,
            genesis_time_ms,
            epoch_index,
            block_height,
            Some(block_info.block_time_ms),
            proposers[3],
            &transaction,
        );

        /*
        Process first block of epoch 3, skipping epoch 2 (i.e. chain halt)

        Should change epoch to 3
        Should pay to proposers for epoch 1
         */

        let epoch_index = GENESIS_EPOCH_INDEX + 3;
        block_height += 1;

        let block_info = helpers::process_and_validate_block_fees(
            &platform,
            genesis_time_ms,
            epoch_index,
            block_height,
            Some(block_info.block_time_ms),
            proposers[3],
            &transaction,
        );

        /*
        Process second block of epoch 3

        Should not change epoch
        Should not pay to proposers
         */

        let epoch_index = GENESIS_EPOCH_INDEX + 3;
        block_height += 1;

        helpers::process_and_validate_block_fees(
            &platform,
            genesis_time_ms,
            epoch_index,
            block_height,
            Some(block_info.block_time_ms),
            proposers[4],
            &transaction,
        );
    }
}
