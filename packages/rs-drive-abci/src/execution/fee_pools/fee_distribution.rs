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

//! Fee Distribution to Proposers.
//!
//! This module defines structs and functions related to distributing fees to proposers.
//!

use crate::error::execution::ExecutionError;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::abci::messages::BlockFees;
use crate::error::Error;
use crate::platform::Platform;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::{Epoch, EpochIndex};
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::ProtocolError;
use drive::drive::batch::drive_op_batch::IdentityOperationType::AddToIdentityBalance;
use drive::drive::batch::DriveOperation::IdentityOperation;
use drive::drive::batch::{DriveOperation, GroveDbOpBatch, SystemOperationType};
use drive::drive::fee_pools::epochs::start_block::StartBlockInfo;
use drive::error::fee::FeeError;
use drive::fee::credits::Credits;
use drive::fee::epoch::GENESIS_EPOCH_INDEX;
use drive::fee_pools::epochs::operations_factory::EpochOperations;
use drive::fee_pools::{
    update_storage_fee_distribution_pool_operation, update_unpaid_epoch_index_operation,
};
use drive::grovedb::{Transaction, TransactionArg};
use drive::{error, grovedb};

/// Struct containing the number of proposers to be paid and the index of the epoch
/// they're to be paid from.
#[derive(PartialEq, Eq, Debug)]
pub struct ProposersPayouts {
    /// Number of proposers to be paid
    pub proposers_paid_count: u16,
    /// Index of last epoch marked as paid
    pub paid_epoch_index: EpochIndex,
}

/// Struct containing the amount of processing and storage fees in the distribution pools
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeesInPools {
    /// Amount of processing fees in the distribution pools
    pub processing_fees: Credits,
    /// Amount of storage fees in the distribution pools
    pub storage_fees: Credits,
}

/// Struct containing info about an epoch containing fees that have not been paid out yet.
#[derive(Default, PartialEq, Eq)]
pub struct UnpaidEpoch {
    /// Index of the current epoch
    pub epoch_index: EpochIndex,
    /// Index of the next unpaid epoch
    pub next_unpaid_epoch_index: EpochIndex,
    /// Block height of the first block in the epoch
    pub start_block_height: u64,
    /// Block height of the first block in next epoch
    pub next_epoch_start_block_height: u64,
    /// Block height of the first block in the epoch
    pub start_block_core_height: u32,
    /// Block height of the first block in next epoch
    pub next_epoch_start_block_core_height: u32,
}

impl UnpaidEpoch {
    /// Counts and returns the number of blocks in the epoch
    fn block_count(&self) -> Result<u64, error::Error> {
        self.next_epoch_start_block_height
            .checked_sub(self.start_block_height)
            .ok_or(error::Error::Fee(FeeError::Overflow(
                "overflow for get_epoch_block_count",
            )))
    }
}

/// Actual number of core blocks per calendar year with DGW v3 is ~200700 (for example 449750 - 249050)
pub const CORE_SUBSIDY_HALVING_INTERVAL: u32 = 210240;

/// ORIGINAL CORE BLOCK DISTRIBUTION
/// STARTS AT 25 Dash
/// Take 60% for Masternodes
/// Take 37.5% of that for Platform
const CORE_GENESIS_BLOCK_SUBSIDY: Credits = 585000000000;

lazy_static! {
    /// The Core reward halving distribution table for 100 years
    /// Yearly decline of production by ~7.1% per year, projected ~18M coins max by year 2050+.
    pub static ref CORE_HALVING_DISTRIBUTION: HashMap<u16, Credits> = {
        let mut distribution = CORE_GENESIS_BLOCK_SUBSIDY;
        (0..100).into_iter().map(|i| {
            let old_distribution = distribution;
            distribution -= distribution / 14;
            (i, old_distribution)
        }).collect()
    };
}

impl<CoreRPCLike> Platform<CoreRPCLike> {
    /// Adds operations to the op batch which distribute fees
    /// from the oldest unpaid epoch pool to proposers.
    ///
    /// Returns `ProposersPayouts` if there are any.
    pub fn add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations(
        &self,
        current_epoch_index: u16,
        cached_current_epoch_start_block_height: Option<u64>,
        cached_current_epoch_start_block_core_height: Option<u32>,
        transaction: &Transaction,
        batch: &mut Vec<DriveOperation>,
    ) -> Result<Option<ProposersPayouts>, Error> {
        let unpaid_epoch = self.find_oldest_epoch_needing_payment(
            current_epoch_index,
            cached_current_epoch_start_block_height,
            cached_current_epoch_start_block_core_height,
            Some(transaction),
        )?;

        let Some(unpaid_epoch) = unpaid_epoch else {
            return Ok(None);
        };

        // Calculate core block reward for the unpaid epoch
        let core_block_rewards = Self::epoch_core_reward_credits_for_distribution(
            unpaid_epoch.start_block_core_height,
            unpaid_epoch.next_epoch_start_block_core_height,
        )?;

        // We must add to the system credits the epoch core block rewards
        // On the Core side we move block rewards every block to asset lock pool
        batch.push(DriveOperation::SystemOperation(
            SystemOperationType::AddToSystemCredits {
                amount: core_block_rewards,
            },
        ));

        let proposers_paid_count = self.add_epoch_pool_to_proposers_payout_operations(
            &unpaid_epoch,
            core_block_rewards,
            transaction,
            batch,
        )?;

        let mut inner_batch = GroveDbOpBatch::new();

        let unpaid_epoch_tree = Epoch::new(unpaid_epoch.epoch_index)?;

        unpaid_epoch_tree.add_mark_as_paid_operations(&mut inner_batch);

        inner_batch.push(update_unpaid_epoch_index_operation(
            unpaid_epoch.next_unpaid_epoch_index,
        ));

        batch.push(DriveOperation::GroveDBOpBatch(inner_batch));

        // We paid to all epoch proposers last block. Since proposers paid count
        // was equal to proposers limit, we paid to 0 proposers this block
        if proposers_paid_count == 0 {
            return Ok(None);
        }

        Ok(Some(ProposersPayouts {
            proposers_paid_count,
            paid_epoch_index: unpaid_epoch.epoch_index,
        }))
    }

    /// Finds and returns the oldest epoch that hasn't been paid out yet.
    fn find_oldest_epoch_needing_payment(
        &self,
        current_epoch_index: u16,
        cached_current_epoch_start_block_height: Option<u64>,
        cached_current_epoch_start_block_core_height: Option<u32>,
        transaction: TransactionArg,
    ) -> Result<Option<UnpaidEpoch>, Error> {
        // Since we are paying for passed epochs there is nothing to do on genesis epoch
        if current_epoch_index == GENESIS_EPOCH_INDEX {
            return Ok(None);
        }

        let unpaid_epoch_index = self.drive.get_unpaid_epoch_index(transaction)?;

        // We pay for previous epochs only
        if unpaid_epoch_index == current_epoch_index {
            return Ok(None);
        }

        let unpaid_epoch = Epoch::new(unpaid_epoch_index)?;

        let start_block_height = self
            .drive
            .get_epoch_start_block_height(&unpaid_epoch, transaction)?;

        let start_block_core_height = self
            .drive
            .get_epoch_start_block_core_height(&unpaid_epoch, transaction)?;

        let next_unpaid_epoch_info = if unpaid_epoch.index == current_epoch_index - 1 {
            // Use cached or committed block height for previous epoch
            let start_block_height = match cached_current_epoch_start_block_height {
                Some(start_block_height) => start_block_height,
                None => {
                    let current_epoch = Epoch::new(current_epoch_index)?;
                    self.drive
                        .get_epoch_start_block_height(&current_epoch, transaction)?
                }
            };

            let start_block_core_height = match cached_current_epoch_start_block_core_height {
                Some(start_block_core_height) => start_block_core_height,
                None => {
                    let current_epoch = Epoch::new(current_epoch_index)?;
                    self.drive
                        .get_epoch_start_block_core_height(&current_epoch, transaction)?
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
            )? {
                // Only possible on epoch change of current epoch, when we have start_block_height batched but not committed yet
                None => {
                    let Some(cached_current_epoch_start_block_height) = cached_current_epoch_start_block_height else {
                        return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "start_block_height must be present in current epoch or cached_next_epoch_start_block_height must be passed",
                        )));
                    };
                    let Some(cached_current_epoch_start_block_core_height) = cached_current_epoch_start_block_core_height else {
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

        Ok(Some(UnpaidEpoch {
            epoch_index: unpaid_epoch_index,
            next_unpaid_epoch_index: next_unpaid_epoch_info.epoch_index,
            start_block_height,
            next_epoch_start_block_height: next_unpaid_epoch_info.start_block_height,
            start_block_core_height,
            next_epoch_start_block_core_height: next_unpaid_epoch_info.start_block_core_height,
        }))
    }

    /// Gets the amount of core reward fees to be distributed for the Epoch.
    pub fn epoch_core_reward_credits_for_distribution(
        epoch_start_block_core_height: u32,
        next_epoch_start_block_core_height: u32,
    ) -> Result<Credits, Error> {
        // Core is halving block rewards every year so we need to pay
        // core block rewards according to halving ratio for the all years during
        // the platform epoch payout period (unpaid epoch)

        // Calculate start and end years for the platform epoch payout period
        // according to start and end core block heights
        let start_core_reward_year =
            (epoch_start_block_core_height / CORE_SUBSIDY_HALVING_INTERVAL) as EpochIndex;
        let end_core_reward_year =
            (next_epoch_start_block_core_height / CORE_SUBSIDY_HALVING_INTERVAL) as EpochIndex;

        let mut total_core_rewards = 0;

        // Calculate block rewards for each core reward year during the platform epoch payout period
        for core_reward_year in start_core_reward_year..=end_core_reward_year {
            // Calculate the block count per core reward year

            let core_reward_year_start_block = if core_reward_year == end_core_reward_year {
                next_epoch_start_block_core_height
            } else {
                (core_reward_year + 1) as u32 * CORE_SUBSIDY_HALVING_INTERVAL
            };

            let core_reward_year_end_block = if core_reward_year == start_core_reward_year {
                epoch_start_block_core_height
            } else {
                core_reward_year as u32 * CORE_SUBSIDY_HALVING_INTERVAL
            };

            let block_count = core_reward_year_start_block - core_reward_year_end_block;

            // Fetch the core block distribution for the corresponding epoch from the distribution table
            // Default to 0 if the core reward year is more than 100 years in the future
            let core_block_distribution_ratio = CORE_HALVING_DISTRIBUTION
                .get(&core_reward_year)
                .unwrap_or(&0);

            // Calculate the core rewards for this epoch and add to the total
            total_core_rewards += block_count as Credits * *core_block_distribution_ratio;
        }

        Ok(total_core_rewards)
    }

    /// Adds operations to the op batch which distribute the fees from an unpaid epoch pool
    /// to the total fees to be paid out to proposers and divides amongst masternode reward shares.
    ///
    /// Returns the number of proposers to be paid out.
    fn add_epoch_pool_to_proposers_payout_operations(
        &self,
        unpaid_epoch: &UnpaidEpoch,
        core_block_rewards: Credits,
        transaction: &Transaction,
        batch: &mut Vec<DriveOperation>,
    ) -> Result<u16, Error> {
        let mut drive_operations = vec![];
        let unpaid_epoch_tree = Epoch::new(unpaid_epoch.epoch_index)?;

        let storage_and_processing_fees = self
            .drive
            .get_epoch_total_credits_for_distribution(&unpaid_epoch_tree, Some(transaction))
            .map_err(Error::Drive)?;

        let total_payouts = storage_and_processing_fees
            .checked_add(core_block_rewards)
            .ok_or_else(|| {
                Error::Execution(ExecutionError::Overflow("overflow when adding reward fees"))
            })?;

        let mut remaining_payouts = total_payouts;

        // Calculate block count
        let unpaid_epoch_block_count = unpaid_epoch.block_count()?;

        let proposers = self
            .drive
            .get_epoch_proposers(&unpaid_epoch_tree, None, Some(transaction))
            .map_err(Error::Drive)?;

        let proposers_len = proposers.len() as u16;

        for (i, (proposer_tx_hash, proposed_block_count)) in proposers.into_iter().enumerate() {
            let i = i as u16;

            let total_masternode_payout = total_payouts
                .checked_mul(proposed_block_count)
                .and_then(|r| r.checked_div(unpaid_epoch_block_count))
                .ok_or(Error::Execution(ExecutionError::Overflow(
                    "overflow when getting masternode reward division",
                )))?;

            let mut masternode_payout_leftover = total_masternode_payout;

            let documents =
                self.get_reward_shares_list_for_masternode(&proposer_tx_hash, Some(transaction))?;

            for document in documents {
                let pay_to_id = document
                    .properties
                    .get_identifier("payToId")
                    .map_err(|e| Error::Protocol(ProtocolError::ValueError(e)))?;

                // TODO this shouldn't be a percentage we need to update masternode share contract
                let share_percentage: u64 = document
                    .properties
                    .get("percentage")
                    .ok_or(Error::Execution(ExecutionError::DriveMissingData(
                        "percentage property is missing".to_string(),
                    )))?
                    .to_integer()
                    .map_err(|_| {
                        Error::Execution(ExecutionError::DriveIncoherence(
                            "percentage property type is not integer",
                        ))
                    })?;

                let share_payout = total_masternode_payout
                    .checked_mul(share_percentage)
                    .and_then(|a| a.checked_div(10000))
                    .ok_or(Error::Execution(ExecutionError::Overflow(
                        "overflow when calculating reward share",
                    )))?;

                // update masternode reward that would be paid later
                masternode_payout_leftover = masternode_payout_leftover
                    .checked_sub(share_payout)
                    .ok_or(Error::Execution(ExecutionError::Overflow(
                    "overflow when subtracting for the masternode share leftover",
                )))?;

                drive_operations.push(IdentityOperation(AddToIdentityBalance {
                    identity_id: pay_to_id.to_buffer(),
                    added_balance: share_payout,
                }));
            }

            remaining_payouts = remaining_payouts
                .checked_sub(total_masternode_payout)
                .ok_or(Error::Execution(ExecutionError::Overflow(
                    "overflow when subtracting for the remaining fees",
                )))?;

            let proposer_payout = if i == proposers_len - 1 {
                remaining_payouts + masternode_payout_leftover
            } else {
                masternode_payout_leftover
            };

            let proposer = proposer_tx_hash.as_slice().try_into().map_err(|_| {
                Error::Execution(ExecutionError::DriveIncoherence(
                    "proposer_tx_hash is not 32 bytes long",
                ))
            })?;

            drive_operations.push(IdentityOperation(AddToIdentityBalance {
                identity_id: proposer,
                added_balance: proposer_payout,
            }));
        }

        let operations = self.drive.convert_drive_operations_to_grove_operations(
            drive_operations,
            &BlockInfo::default(),
            Some(transaction),
        )?;

        batch.push(DriveOperation::GroveDBOpBatch(operations));

        Ok(proposers_len)
    }

    /// Adds operations to an op batch which update total storage fees
    /// for the epoch considering fees from a new block.
    ///
    /// Returns `FeesInPools`
    pub fn add_distribute_block_fees_into_pools_operations(
        &self,
        current_epoch: &Epoch,
        block_fees: &BlockFees,
        cached_aggregated_storage_fees: Option<Credits>,
        transaction: TransactionArg,
        batch: &mut Vec<DriveOperation>,
    ) -> Result<FeesInPools, Error> {
        // update epochs pool processing fees
        let epoch_processing_fees = self
            .drive
            .get_epoch_processing_credits_for_distribution(current_epoch, transaction)
            .or_else(|e| match e {
                // Handle epoch change when storage fees are not set yet
                error::Error::GroveDB(grovedb::Error::PathKeyNotFound(_)) => Ok(0u64),
                _ => Err(e),
            })?;

        let total_processing_fees = epoch_processing_fees + block_fees.processing_fee;

        batch.push(DriveOperation::GroveDBOperation(
            current_epoch.update_processing_fee_pool_operation(total_processing_fees)?,
        ));

        // update storage fee pool
        let storage_distribution_credits_in_fee_pool = match cached_aggregated_storage_fees {
            None => self
                .drive
                .get_storage_fees_from_distribution_pool(transaction)?,
            Some(storage_fees) => storage_fees,
        };

        let total_storage_fees = storage_distribution_credits_in_fee_pool + block_fees.storage_fee;

        batch.push(DriveOperation::GroveDBOperation(
            update_storage_fee_distribution_pool_operation(
                storage_distribution_credits_in_fee_pool + block_fees.storage_fee,
            )?,
        ));

        Ok(FeesInPools {
            processing_fees: total_processing_fees,
            storage_fees: total_storage_fees,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use drive::common::helpers::identities::create_test_masternode_identities_and_add_them_as_epoch_block_proposers;

    mod add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations {
        use crate::test::helpers::setup::TestPlatformBuilder;

        use super::*;

        use drive::error::Error as DriveError;

        #[test]
        fn test_nothing_to_distribute_if_there_is_no_epochs_needing_payment() {
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let current_epoch_index = 0;

            let mut batch = vec![];

            let proposers_payouts = platform
                .add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations(
                    current_epoch_index,
                    None,
                    None,
                    &transaction,
                    &mut batch,
                )
                .expect("should distribute fees");

            assert!(proposers_payouts.is_none());
        }

        #[test]
        fn test_mark_epoch_as_paid_and_update_next_update_epoch_index_if_all_proposers_paid() {
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            // Create masternode reward shares contract
            platform.create_mn_shares_contract(Some(&transaction));

            let proposers_count = 150;
            let processing_fees = 100000000;
            let storage_fees = 10000000;

            let unpaid_epoch = Epoch::new(0).unwrap();
            let current_epoch = Epoch::new(1).unwrap();

            let mut batch = GroveDbOpBatch::new();

            unpaid_epoch.add_init_current_operations(1.0, 1, 1, 1, &mut batch);

            batch.push(
                unpaid_epoch
                    .update_processing_fee_pool_operation(processing_fees)
                    .expect("should add operation"),
            );

            batch.push(
                unpaid_epoch
                    .update_storage_fee_pool_operation(storage_fees)
                    .expect("should add operation"),
            );

            current_epoch.add_init_current_operations(
                1.0,
                proposers_count as u64 + 1,
                3,
                2,
                &mut batch,
            );

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let proposers = create_test_masternode_identities_and_add_them_as_epoch_block_proposers(
                &platform.drive,
                &unpaid_epoch,
                proposers_count,
                Some(65), //random number
                Some(&transaction),
            );

            let mut batch = vec![];

            let proposer_payouts = platform
                .add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations(
                    current_epoch.index,
                    None,
                    None,
                    &transaction,
                    &mut batch,
                )
                .expect("should distribute fees");

            platform
                .drive
                .apply_drive_operations(batch, true, &BlockInfo::default(), Some(&transaction))
                .expect("should apply batch");

            assert!(matches!(
                proposer_payouts,
                Some(ProposersPayouts {
                    proposers_paid_count: p,
                    paid_epoch_index: 0,
                }) if p == proposers_count
            ));

            let next_unpaid_epoch_index = platform
                .drive
                .get_unpaid_epoch_index(Some(&transaction))
                .expect("should get unpaid epoch index");

            assert_eq!(next_unpaid_epoch_index, current_epoch.index);

            // check we've removed proposers tree
            let result = platform.drive.get_epochs_proposer_block_count(
                &unpaid_epoch,
                &proposers[0],
                Some(&transaction),
            );

            assert!(matches!(
                result,
                Err(DriveError::GroveDB(
                    grovedb::Error::PathParentLayerNotFound(_)
                ))
            ));
        }
    }

    mod find_oldest_epoch_needing_payment {
        use crate::test::helpers::setup::TestPlatformBuilder;

        use super::*;

        #[test]
        fn test_no_epoch_to_pay_on_genesis_epoch() {
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let unpaid_epoch = platform
                .find_oldest_epoch_needing_payment(
                    GENESIS_EPOCH_INDEX,
                    None,
                    None,
                    Some(&transaction),
                )
                .expect("should find nothing");

            assert!(unpaid_epoch.is_none());
        }

        #[test]
        fn test_no_epoch_to_pay_if_oldest_unpaid_epoch_is_current_epoch() {
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
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let unpaid_epoch = platform
                .find_oldest_epoch_needing_payment(
                    current_epoch_index,
                    None,
                    None,
                    Some(&transaction),
                )
                .expect("should find nothing");

            assert!(unpaid_epoch.is_none());
        }

        #[test]
        fn test_use_cached_current_start_block_height_as_end_block_if_unpaid_epoch_is_previous() {
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
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let cached_current_epoch_start_block_height = Some(2);

            let cached_current_epoch_start_block_core_height = Some(2);

            let unpaid_epoch = platform
                .find_oldest_epoch_needing_payment(
                    current_epoch_index,
                    cached_current_epoch_start_block_height,
                    cached_current_epoch_start_block_core_height,
                    Some(&transaction),
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
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let unpaid_epoch = platform
                .find_oldest_epoch_needing_payment(
                    current_epoch_index,
                    None,
                    None,
                    Some(&transaction),
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
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let unpaid_epoch = platform
                .find_oldest_epoch_needing_payment(
                    current_epoch_index,
                    None,
                    None,
                    Some(&transaction),
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
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let cached_current_epoch_start_block_height = Some(2);
            let cached_current_epoch_start_block_core_height = Some(2);

            let unpaid_epoch = platform
                .find_oldest_epoch_needing_payment(
                    current_epoch_index,
                    cached_current_epoch_start_block_height,
                    cached_current_epoch_start_block_core_height,
                    Some(&transaction),
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
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let unpaid_epoch = platform.find_oldest_epoch_needing_payment(
                current_epoch_index,
                None,
                None,
                Some(&transaction),
            );

            assert!(matches!(
                unpaid_epoch,
                Err(Error::Execution(ExecutionError::CorruptedCodeExecution(_)))
            ));
        }
    }

    mod add_epoch_pool_to_proposers_payout_operations {
        use super::*;
        use crate::test::helpers::{
            fee_pools::create_test_masternode_share_identities_and_documents,
            setup::TestPlatformBuilder,
        };
        use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
        use rust_decimal::Decimal;
        use rust_decimal_macros::dec;

        #[test]
        fn test_payout_to_proposers() {
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            // Create masternode reward shares contract
            let contract = platform.create_mn_shares_contract(Some(&transaction));

            let proposers_count = 10u16;
            let processing_fees = 10000;
            let storage_fees = 10000;

            let unpaid_epoch_tree = Epoch::new(0).unwrap();
            let next_epoch_tree = Epoch::new(1).unwrap();

            let mut batch = GroveDbOpBatch::new();

            unpaid_epoch_tree.add_init_current_operations(1.0, 1, 1, 1, &mut batch);

            batch.push(
                unpaid_epoch_tree
                    .update_processing_fee_pool_operation(processing_fees)
                    .expect("should add operation"),
            );

            batch.push(
                unpaid_epoch_tree
                    .update_storage_fee_pool_operation(storage_fees)
                    .expect("should add operation"),
            );

            next_epoch_tree.add_init_current_operations(
                1.0,
                proposers_count as u64 + 1,
                1,
                10,
                &mut batch,
            );

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let pro_tx_hashes =
                create_test_masternode_identities_and_add_them_as_epoch_block_proposers(
                    &platform.drive,
                    &unpaid_epoch_tree,
                    proposers_count,
                    Some(68), //random number
                    Some(&transaction),
                );

            let share_identities_and_documents =
                create_test_masternode_share_identities_and_documents(
                    &platform.drive,
                    &contract,
                    &pro_tx_hashes,
                    Some(55),
                    Some(&transaction),
                );

            let mut batch = vec![];

            let unpaid_epoch = UnpaidEpoch {
                epoch_index: 0,
                start_block_height: 1,
                next_epoch_start_block_height: 11,
                start_block_core_height: 1,
                next_unpaid_epoch_index: 0,
                next_epoch_start_block_core_height: 1,
            };

            let proposers_paid_count = platform
                .add_epoch_pool_to_proposers_payout_operations(
                    &unpaid_epoch,
                    0,
                    &transaction,
                    &mut batch,
                )
                .expect("should distribute fees");

            platform
                .drive
                .apply_drive_operations(batch, true, &BlockInfo::default(), Some(&transaction))
                .expect("should apply batch");

            assert_eq!(proposers_paid_count, 10);

            // check we paid 500 to every mn identity
            let paid_mn_identities_balances = platform
                .drive
                .fetch_identities_balances(&pro_tx_hashes, Some(&transaction))
                .expect("expected to get identities");

            let total_fees = Decimal::from(storage_fees + processing_fees);

            let masternode_reward = total_fees / Decimal::from(proposers_count);

            let shares_percentage_with_precision: u64 = share_identities_and_documents[0]
                .1
                .properties
                .get_integer("percentage")
                .expect("should have percentage field");

            let shares_percentage = Decimal::from(shares_percentage_with_precision) / dec!(10000);

            let payout_credits = masternode_reward * shares_percentage;

            let payout_credits: u64 = payout_credits.try_into().expect("should convert to u64");

            for (_, paid_mn_identity_balance) in paid_mn_identities_balances {
                assert_eq!(paid_mn_identity_balance, payout_credits);
            }

            let share_identities = share_identities_and_documents
                .iter()
                .map(|(identity, _)| identity.id.to_buffer())
                .collect();

            let refetched_share_identities_balances = platform
                .drive
                .fetch_identities_balances(&share_identities, Some(&transaction))
                .expect("expected to get identities");

            for (_, balance) in refetched_share_identities_balances {
                assert_eq!(balance, payout_credits);
            }
        }
    }

    mod add_distribute_block_fees_into_pools_operations {
        use crate::test::helpers::setup::TestPlatformBuilder;

        use super::*;

        #[test]
        fn test_distribute_block_fees_into_uncommitted_epoch_on_epoch_change() {
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let current_epoch_tree = Epoch::new(1).unwrap();

            let mut batch = vec![];

            let mut inner_batch = GroveDbOpBatch::new();

            current_epoch_tree.add_init_current_operations(1.0, 1, 1, 1, &mut inner_batch);

            batch.push(DriveOperation::GroveDBOpBatch(inner_batch));

            let processing_fees = 1000000;
            let storage_fees = 2000000;

            platform
                .add_distribute_block_fees_into_pools_operations(
                    &current_epoch_tree,
                    &BlockFees::from_fees(storage_fees, processing_fees),
                    None,
                    Some(&transaction),
                    &mut batch,
                )
                .expect("should distribute fees into pools");

            platform
                .drive
                .apply_drive_operations(batch, true, &BlockInfo::default(), Some(&transaction))
                .expect("should apply batch");

            let stored_processing_fee_credits = platform
                .drive
                .get_epoch_processing_credits_for_distribution(
                    &current_epoch_tree,
                    Some(&transaction),
                )
                .expect("should get processing fees");

            let stored_storage_fee_credits = platform
                .drive
                .get_storage_fees_from_distribution_pool(Some(&transaction))
                .expect("should get storage fee pool");

            assert_eq!(stored_processing_fee_credits, processing_fees);
            assert_eq!(stored_storage_fee_credits, storage_fees);
        }

        #[test]
        fn test_distribute_block_fees_into_pools() {
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let current_epoch_tree = Epoch::new(1).unwrap();

            let mut batch = GroveDbOpBatch::new();

            current_epoch_tree.add_init_current_operations(1.0, 1, 1, 1, &mut batch);

            // Apply new pool structure
            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let mut batch = vec![];

            let processing_fees = 1000000;
            let storage_fees = 2000000;

            platform
                .add_distribute_block_fees_into_pools_operations(
                    &current_epoch_tree,
                    &BlockFees::from_fees(storage_fees, processing_fees),
                    None,
                    Some(&transaction),
                    &mut batch,
                )
                .expect("should distribute fees into pools");

            platform
                .drive
                .apply_drive_operations(batch, true, &BlockInfo::default(), Some(&transaction))
                .expect("should apply batch");

            let stored_processing_fee_credits = platform
                .drive
                .get_epoch_processing_credits_for_distribution(
                    &current_epoch_tree,
                    Some(&transaction),
                )
                .expect("should get processing fees");

            let stored_storage_fee_credits = platform
                .drive
                .get_storage_fees_from_distribution_pool(Some(&transaction))
                .expect("should get storage fee pool");

            assert_eq!(stored_processing_fee_credits, processing_fees);
            assert_eq!(stored_storage_fee_credits, storage_fees);
        }
    }
}
