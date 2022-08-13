use crate::abci::messages::FeesAggregate;
use crate::error::execution::ExecutionError;

use crate::error::Error;
use crate::platform::Platform;
use rs_drive::drive::batch::GroveDbOpBatch;
use rs_drive::drive::fee_pools::epochs::constants::GENESIS_EPOCH_INDEX;
use rs_drive::error::fee::FeeError;
use rs_drive::fee_pools::epochs::Epoch;
use rs_drive::fee_pools::{
    update_storage_fee_distribution_pool_operation, update_unpaid_epoch_index_operation,
};
use rs_drive::grovedb::TransactionArg;
use rs_drive::{error, grovedb};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub struct ProposersPayouts {
    pub proposers_paid_count: u16,
    pub paid_epoch_index: u16,
}

pub struct FeesInPools {
    pub processing_fees: u64,
    pub storage_fees: u64,
}

#[derive(Default)]
pub struct UnpaidEpoch {
    pub epoch_index: u16,
    pub start_block_height: u64,
    pub end_block_height: u64,
    pub next_unpaid_epoch_index: u16,
}

impl UnpaidEpoch {
    fn block_count(&self) -> Result<u64, error::Error> {
        self.end_block_height
            .checked_sub(self.start_block_height)
            .ok_or(error::Error::Fee(FeeError::Overflow(
                "overflow for get_epoch_block_count",
            )))
    }
}

impl Platform {
    pub fn add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations(
        &self,
        current_epoch_index: u16,
        cached_current_epoch_start_block_height: Option<u64>,
        transaction: TransactionArg,
        batch: &mut GroveDbOpBatch,
    ) -> Result<Option<ProposersPayouts>, Error> {
        let unpaid_epoch = self.find_oldest_epoch_needing_payment(
            current_epoch_index,
            cached_current_epoch_start_block_height,
            transaction,
        )?;

        if unpaid_epoch.is_none() {
            return Ok(None);
        }

        let unpaid_epoch = unpaid_epoch.unwrap();

        // Process more proposers at once if we have many unpaid epochs in past
        let proposers_limit: u16 = (current_epoch_index - unpaid_epoch.epoch_index) * 50;

        let proposers_paid_count = self.add_epoch_pool_to_proposers_payout_operations(
            &unpaid_epoch,
            proposers_limit,
            transaction,
            batch,
        )?;

        // if less then a limit paid then mark the epoch pool as paid
        if proposers_paid_count < proposers_limit {
            let unpaid_epoch_tree = Epoch::new(unpaid_epoch.epoch_index);

            unpaid_epoch_tree.add_mark_as_paid_operations(batch);

            batch.push(update_unpaid_epoch_index_operation(
                unpaid_epoch.next_unpaid_epoch_index,
            ));
        }

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

    fn find_oldest_epoch_needing_payment(
        &self,
        current_epoch_index: u16,
        cached_current_epoch_start_block_height: Option<u64>,
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

        let unpaid_epoch = Epoch::new(unpaid_epoch_index);

        let start_block_height = self
            .drive
            .get_epoch_start_block_height(&unpaid_epoch, transaction)?;

        let (next_unpaid_epoch_index, end_block_height) = if unpaid_epoch.index
            == current_epoch_index - 1
        {
            // Use cached or committed block height for previous epoch
            let start_block_height = match cached_current_epoch_start_block_height {
                Some(start_block_height) => start_block_height,
                None => {
                    let current_epoch = Epoch::new(current_epoch_index);
                    self.drive
                        .get_epoch_start_block_height(&current_epoch, transaction)?
                }
            };

            (current_epoch_index, start_block_height)
        } else {
            // Find a next epoch with start block height if unpaid epoch was more than one epoch ago
            match self
                .drive
                .get_first_epoch_start_block_height_between_epochs(
                    unpaid_epoch.index,
                    current_epoch_index,
                    transaction,
                )? {
                // Only possible on epoch change of current epoch, when we have start_block_height batched but not committed yet
                None => match cached_current_epoch_start_block_height {
                    None => {
                        return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "start_block_height must be present in current epoch or cached_next_epoch_start_block_height must be passed",
                        )));
                    }
                    Some(cached_current_epoch_start_block_height) => {
                        (current_epoch_index, cached_current_epoch_start_block_height)
                    }
                },
                Some(next_start_block_info) => next_start_block_info,
            }
        };

        // Use cached current epoch start block height only if we pay for the previous epoch

        Ok(Some(UnpaidEpoch {
            epoch_index: unpaid_epoch_index,
            next_unpaid_epoch_index,
            start_block_height,
            end_block_height,
        }))
    }

    fn add_epoch_pool_to_proposers_payout_operations(
        &self,
        unpaid_epoch: &UnpaidEpoch,
        proposers_limit: u16,
        transaction: TransactionArg,
        batch: &mut GroveDbOpBatch,
    ) -> Result<u16, Error> {
        let unpaid_epoch_tree = Epoch::new(unpaid_epoch.epoch_index);

        let total_fees = self
            .drive
            .get_epoch_total_credits_for_distribution(&unpaid_epoch_tree, transaction)
            .map_err(Error::Drive)?;

        let total_fees = Decimal::from(total_fees);

        // Calculate block count
        let unpaid_epoch_block_count = Decimal::from(unpaid_epoch.block_count()?);

        let proposers = self
            .drive
            .get_epoch_proposers(&unpaid_epoch_tree, proposers_limit, transaction)
            .map_err(Error::Drive)?;

        let proposers_len = proposers.len() as u16;

        let mut fee_leftovers = dec!(0.0);

        for (i, (proposer_tx_hash, proposed_block_count)) in proposers.iter().enumerate() {
            let i = i as u16;
            let proposed_block_count = Decimal::from(*proposed_block_count);

            let mut masternode_reward =
                (total_fees * proposed_block_count) / unpaid_epoch_block_count;

            let documents =
                self.get_reward_shares_list_for_masternode(proposer_tx_hash, transaction)?;

            for document in documents {
                let pay_to_id = document
                    .properties
                    .get("payToId")
                    .ok_or(Error::Execution(ExecutionError::DriveMissingData(
                        "payToId property is missing",
                    )))?
                    .as_bytes()
                    .ok_or(Error::Execution(ExecutionError::DriveIncoherence(
                        "payToId property type is not bytes",
                    )))?;

                // TODO this shouldn't be a percentage we need to update masternode share contract
                let share_percentage_integer: i64 = document
                    .properties
                    .get("percentage")
                    .ok_or(Error::Execution(ExecutionError::DriveMissingData(
                        "percentage property is missing",
                    )))?
                    .as_integer()
                    .ok_or(Error::Execution(ExecutionError::DriveIncoherence(
                        "percentage property type is not integer",
                    )))?
                    .try_into()
                    .map_err(|_| {
                        Error::Execution(ExecutionError::Overflow(
                            "percentage property cannot be converted to i64",
                        ))
                    })?;

                let share_percentage = Decimal::from(share_percentage_integer) / dec!(10000);

                let reward = masternode_reward * share_percentage;

                let reward_floored = reward.floor();

                // update masternode reward that would be paid later
                masternode_reward -= reward_floored;

                let reward_floored: u64 = reward_floored.try_into().map_err(|_| {
                    Error::Execution(ExecutionError::Overflow(
                        "can't convert reward to i64 from Decimal",
                    ))
                })?;

                self.add_pay_reward_to_identity_operations(
                    pay_to_id,
                    reward_floored,
                    transaction,
                    batch,
                )?;
            }

            // Since balance is an integer, we collect rewards remainder
            // and add leftovers to the latest proposer of the chunk
            let masternode_reward_floored = masternode_reward.floor();

            fee_leftovers += masternode_reward - masternode_reward_floored;

            let masternode_reward_given = if i == proposers_len - 1 {
                masternode_reward_floored + fee_leftovers
            } else {
                masternode_reward_floored
            };

            // Convert to integer, since identity balance is u64
            let masternode_reward_given: u64 =
                masternode_reward_given.floor().try_into().map_err(|_| {
                    Error::Execution(ExecutionError::Overflow(
                        "can't convert reward to i64 from Decimal",
                    ))
                })?;

            self.add_pay_reward_to_identity_operations(
                proposer_tx_hash,
                masternode_reward_given,
                transaction,
                batch,
            )?;
        }

        // remove proposers we've paid out
        let proposer_pro_tx_hashes: Vec<Vec<u8>> =
            proposers.iter().map(|(hash, _)| hash.clone()).collect();

        unpaid_epoch_tree.add_delete_proposers_operations(proposer_pro_tx_hashes, batch);

        Ok(proposers_len)
    }

    fn add_pay_reward_to_identity_operations(
        &self,
        id: &[u8],
        reward: u64,
        transaction: TransactionArg,
        batch: &mut GroveDbOpBatch,
    ) -> Result<(), Error> {
        // We don't need additional verification, since we ensure an identity
        // existence in the data contract triggers in DPP
        let (mut identity, storage_flags) = self.drive.fetch_identity(id, transaction)?;

        identity.balance = identity.balance.checked_add(reward).ok_or(Error::Execution(ExecutionError::Overflow("pay reward overflow")))?;

        self.drive
            .add_insert_identity_operations(identity, storage_flags, batch)
            .map_err(Error::Drive)
    }

    pub fn add_distribute_block_fees_into_pools_operations(
        &self,
        current_epoch: &Epoch,
        block_fees: &FeesAggregate,
        cached_aggregated_storage_fees: Option<u64>,
        transaction: TransactionArg,
        batch: &mut GroveDbOpBatch,
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

        let total_processing_fees = epoch_processing_fees + block_fees.processing_fees;

        batch.push(
            current_epoch
                .update_processing_credits_for_distribution_operation(total_processing_fees),
        );

        // update storage fee pool
        let storage_distribution_credits_in_fee_pool = match cached_aggregated_storage_fees {
            None => self
                .drive
                .get_aggregate_storage_fees_from_distribution_pool(transaction)?,
            Some(storage_fees) => storage_fees,
        };

        let total_storage_fees = storage_distribution_credits_in_fee_pool + block_fees.storage_fees;

        batch.push(update_storage_fee_distribution_pool_operation(
            storage_distribution_credits_in_fee_pool + block_fees.storage_fees,
        ));

        Ok(FeesInPools {
            processing_fees: total_processing_fees,
            storage_fees: total_storage_fees,
        })
    }
}

#[cfg(test)]
mod tests {
    mod add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations {
        use crate::common::helpers::setup::setup_platform_with_initial_state_structure;
        use rs_drive::common::helpers::identities::create_test_masternode_identities_and_add_them_as_epoch_block_proposers;
        use rs_drive::drive::batch::GroveDbOpBatch;
        use rs_drive::drive::fee_pools::epochs::constants::GENESIS_EPOCH_INDEX;
        use rs_drive::error::Error;
        use rs_drive::fee_pools::epochs::Epoch;
        use rs_drive::grovedb;

        #[test]
        fn test_nothing_to_distribute_if_there_is_no_epochs_needing_payment() {
            let platform = setup_platform_with_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let current_epoch_index = 0;

            let mut batch = GroveDbOpBatch::new();

            let proposers_payouts = platform
                .add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations(
                    current_epoch_index,
                    None,
                    Some(&transaction),
                    &mut batch,
                )
                .expect("should distribute fees");

            assert!(proposers_payouts.is_none());
        }

        #[test]
        fn test_set_proposers_limit_50_for_one_unpaid_epoch() {
            let platform = setup_platform_with_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            // Create masternode reward shares contract
            platform.create_mn_shares_contract(Some(&transaction));

            // Create epochs

            let unpaid_epoch_tree_0 = Epoch::new(GENESIS_EPOCH_INDEX);

            let current_epoch_index = GENESIS_EPOCH_INDEX + 1;

            let epoch_tree_1 = Epoch::new(current_epoch_index);

            let mut batch = GroveDbOpBatch::new();

            unpaid_epoch_tree_0.add_init_current_operations(1.0, 1, 1, &mut batch);

            batch.push(
                unpaid_epoch_tree_0.update_processing_credits_for_distribution_operation(10000),
            );

            let proposers_count = 100u16;

            epoch_tree_1.add_init_current_operations(
                1.0,
                proposers_count as u64 + 1,
                2,
                &mut batch,
            );

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            create_test_masternode_identities_and_add_them_as_epoch_block_proposers(
                &platform.drive,
                &unpaid_epoch_tree_0,
                proposers_count,
                Some(&transaction),
            );

            let mut batch = GroveDbOpBatch::new();

            let proposer_payouts = platform
                .add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations(
                    current_epoch_index,
                    None,
                    Some(&transaction),
                    &mut batch,
                )
                .expect("should distribute fees");

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            match proposer_payouts {
                None => assert!(false, "proposers should be paid"),
                Some(payouts) => {
                    assert_eq!(payouts.proposers_paid_count, 50);
                    assert_eq!(payouts.paid_epoch_index, 0);
                }
            }
        }

        #[test]
        fn test_increased_proposers_limit_to_100_for_two_unpaid_epochs() {
            let platform = setup_platform_with_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            // Create masternode reward shares contract
            platform.create_mn_shares_contract(Some(&transaction));

            // Create epochs

            let unpaid_epoch_tree_0 = Epoch::new(GENESIS_EPOCH_INDEX);
            let unpaid_epoch_tree_1 = Epoch::new(GENESIS_EPOCH_INDEX + 1);

            let current_epoch_index = GENESIS_EPOCH_INDEX + 2;

            let epoch_tree_2 = Epoch::new(current_epoch_index);

            let mut batch = GroveDbOpBatch::new();

            unpaid_epoch_tree_0.add_init_current_operations(1.0, 1, 1, &mut batch);

            batch.push(
                unpaid_epoch_tree_0.update_processing_credits_for_distribution_operation(10000),
            );

            let proposers_count = 100u16;

            unpaid_epoch_tree_1.add_init_current_operations(
                1.0,
                proposers_count as u64 + 1,
                2,
                &mut batch,
            );

            epoch_tree_2.add_init_current_operations(
                1.0,
                proposers_count as u64 * 2 + 1,
                3,
                &mut batch,
            );

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            create_test_masternode_identities_and_add_them_as_epoch_block_proposers(
                &platform.drive,
                &unpaid_epoch_tree_0,
                proposers_count,
                Some(&transaction),
            );

            create_test_masternode_identities_and_add_them_as_epoch_block_proposers(
                &platform.drive,
                &unpaid_epoch_tree_1,
                proposers_count,
                Some(&transaction),
            );

            let mut batch = GroveDbOpBatch::new();

            let proposer_payouts = platform
                .add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations(
                    current_epoch_index,
                    None,
                    Some(&transaction),
                    &mut batch,
                )
                .expect("should distribute fees");

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            match proposer_payouts {
                None => assert!(false, "proposers should be paid"),
                Some(payouts) => {
                    assert_eq!(payouts.proposers_paid_count, 100);
                    assert_eq!(payouts.paid_epoch_index, 0);
                }
            }
        }

        #[test]
        fn test_increased_proposers_limit_to_150_for_three_unpaid_epochs() {
            let platform = setup_platform_with_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            // Create masternode reward shares contract
            platform.create_mn_shares_contract(Some(&transaction));

            // Create epochs

            let unpaid_epoch_tree_0 = Epoch::new(GENESIS_EPOCH_INDEX);
            let unpaid_epoch_tree_1 = Epoch::new(GENESIS_EPOCH_INDEX + 1);
            let unpaid_epoch_tree_2 = Epoch::new(GENESIS_EPOCH_INDEX + 2);

            let current_epoch_index = GENESIS_EPOCH_INDEX + 3;

            let epoch_tree_3 = Epoch::new(current_epoch_index);

            let mut batch = GroveDbOpBatch::new();

            unpaid_epoch_tree_0.add_init_current_operations(1.0, 1, 1, &mut batch);

            batch.push(
                unpaid_epoch_tree_0.update_processing_credits_for_distribution_operation(10000),
            );

            let proposers_count = 200u16;

            unpaid_epoch_tree_1.add_init_current_operations(
                1.0,
                proposers_count as u64 + 1,
                2,
                &mut batch,
            );

            unpaid_epoch_tree_2.add_init_current_operations(
                1.0,
                proposers_count as u64 * 2 + 1,
                3,
                &mut batch,
            );

            epoch_tree_3.add_init_current_operations(
                1.0,
                proposers_count as u64 * 3 + 1,
                3,
                &mut batch,
            );

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            create_test_masternode_identities_and_add_them_as_epoch_block_proposers(
                &platform.drive,
                &unpaid_epoch_tree_0,
                proposers_count,
                Some(&transaction),
            );

            create_test_masternode_identities_and_add_them_as_epoch_block_proposers(
                &platform.drive,
                &unpaid_epoch_tree_1,
                proposers_count,
                Some(&transaction),
            );

            create_test_masternode_identities_and_add_them_as_epoch_block_proposers(
                &platform.drive,
                &unpaid_epoch_tree_2,
                proposers_count,
                Some(&transaction),
            );

            let mut batch = GroveDbOpBatch::new();

            let proposer_payouts = platform
                .add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations(
                    current_epoch_index,
                    None,
                    Some(&transaction),
                    &mut batch,
                )
                .expect("should distribute fees");

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            match proposer_payouts {
                None => assert!(false, "proposers should be paid"),
                Some(payouts) => {
                    assert_eq!(payouts.proposers_paid_count, 150);
                    assert_eq!(payouts.paid_epoch_index, 0);
                }
            }
        }

        #[test]
        fn test_mark_epoch_as_paid_and_update_next_update_epoch_index_if_all_proposers_paid() {
            let platform = setup_platform_with_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            // Create masternode reward shares contract
            platform.create_mn_shares_contract(Some(&transaction));

            let proposers_count = 10;
            let processing_fees = 10000;
            let storage_fees = 10000;

            let unpaid_epoch = Epoch::new(0);
            let current_epoch = Epoch::new(1);

            let mut batch = GroveDbOpBatch::new();

            unpaid_epoch.add_init_current_operations(1.0, 1, 1, &mut batch);

            batch.push(
                unpaid_epoch.update_processing_credits_for_distribution_operation(processing_fees),
            );

            batch
                .push(unpaid_epoch.update_storage_credits_for_distribution_operation(storage_fees));

            current_epoch.add_init_current_operations(1.0, 2, 2, &mut batch);

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let proposers = create_test_masternode_identities_and_add_them_as_epoch_block_proposers(
                &platform.drive,
                &unpaid_epoch,
                proposers_count,
                Some(&transaction),
            );

            let mut batch = GroveDbOpBatch::new();

            let proposer_payouts = platform
                .add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations(
                    current_epoch.index,
                    None,
                    Some(&transaction),
                    &mut batch,
                )
                .expect("should distribute fees");

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            match proposer_payouts {
                None => assert!(false, "proposers should be paid"),
                Some(payouts) => {
                    assert_eq!(payouts.proposers_paid_count, proposers_count);
                    assert_eq!(payouts.paid_epoch_index, 0);
                }
            }

            let next_unpaid_epoch_index = platform
                .drive
                .get_unpaid_epoch_index(Some(&transaction))
                .expect("should get unpaid epoch index");

            assert_eq!(next_unpaid_epoch_index, current_epoch.index);

            // check we've removed proposers tree
            match platform.drive.get_epochs_proposer_block_count(
                &unpaid_epoch,
                &proposers[0],
                Some(&transaction),
            ) {
                Ok(_) => assert!(false, "expect tree not exists"),
                Err(e) => match e {
                    Error::GroveDB(grovedb::Error::PathNotFound(_)) => assert!(true),
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_mark_epoch_as_paid_and_update_next_update_epoch_index_if_all_50_proposers_were_paid_last_block(
        ) {
            let platform = setup_platform_with_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            // Create masternode reward shares contract
            platform.create_mn_shares_contract(Some(&transaction));

            let proposers_count = 50;
            let processing_fees = 10000;
            let storage_fees = 10000;

            let unpaid_epoch = Epoch::new(0);
            let current_epoch = Epoch::new(1);

            let mut batch = GroveDbOpBatch::new();

            unpaid_epoch.add_init_current_operations(1.0, 1, 1, &mut batch);

            batch.push(
                unpaid_epoch.update_processing_credits_for_distribution_operation(processing_fees),
            );

            batch
                .push(unpaid_epoch.update_storage_credits_for_distribution_operation(storage_fees));

            current_epoch.add_init_current_operations(1.0, 2, 2, &mut batch);

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let proposers = create_test_masternode_identities_and_add_them_as_epoch_block_proposers(
                &platform.drive,
                &unpaid_epoch,
                proposers_count,
                Some(&transaction),
            );

            let mut batch = GroveDbOpBatch::new();

            let proposer_payouts = platform
                .add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations(
                    current_epoch.index,
                    None,
                    Some(&transaction),
                    &mut batch,
                )
                .expect("should distribute fees");

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            match proposer_payouts {
                None => assert!(false, "proposers should be paid"),
                Some(payouts) => {
                    assert_eq!(payouts.proposers_paid_count, proposers_count);
                    assert_eq!(payouts.paid_epoch_index, 0);
                }
            }

            // The Epoch 0 should still not marked as paid because proposers count == proposers limit
            let next_unpaid_epoch_index = platform
                .drive
                .get_unpaid_epoch_index(Some(&transaction))
                .expect("should get unpaid epoch index");

            assert_eq!(next_unpaid_epoch_index, unpaid_epoch.index);

            // Process one more block

            let mut batch = GroveDbOpBatch::new();

            let proposer_payouts = platform
                .add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations(
                    current_epoch.index,
                    None,
                    Some(&transaction),
                    &mut batch,
                )
                .expect("should distribute fees");

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let next_unpaid_epoch_index = platform
                .drive
                .get_unpaid_epoch_index(Some(&transaction))
                .expect("should get unpaid epoch index");

            assert_eq!(next_unpaid_epoch_index, current_epoch.index);

            // check we've removed proposers tree
            match platform.drive.get_epochs_proposer_block_count(
                &unpaid_epoch,
                &proposers[0],
                Some(&transaction),
            ) {
                Ok(_) => assert!(false, "expect tree not exists"),
                Err(e) => match e {
                    Error::GroveDB(grovedb::Error::PathNotFound(_)) => assert!(true),
                    _ => assert!(false, "invalid error type"),
                },
            }
        }
    }

    mod find_oldest_epoch_needing_payment {
        use crate::common::helpers::setup::setup_platform_with_initial_state_structure;
        use crate::error::execution::ExecutionError;
        use crate::error::Error;
        use rs_drive::drive::batch::GroveDbOpBatch;
        use rs_drive::drive::fee_pools::epochs::constants::GENESIS_EPOCH_INDEX;
        use rs_drive::fee_pools::epochs::Epoch;
        use rs_drive::fee_pools::update_unpaid_epoch_index_operation;

        #[test]
        fn test_no_epoch_to_pay_on_genesis_epoch() {
            let platform = setup_platform_with_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let unpaid_epoch = platform
                .find_oldest_epoch_needing_payment(GENESIS_EPOCH_INDEX, None, Some(&transaction))
                .expect("should find nothing");

            assert!(unpaid_epoch.is_none());
        }

        #[test]
        fn test_no_epoch_to_pay_if_oldest_unpaid_epoch_is_current_epoch() {
            let platform = setup_platform_with_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let epoch_0_tree = Epoch::new(GENESIS_EPOCH_INDEX);

            let current_epoch_index = GENESIS_EPOCH_INDEX + 1;

            let epoch_1_tree = Epoch::new(current_epoch_index);

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch_0_tree.update_start_block_height_operation(1));
            batch.push(epoch_1_tree.update_start_block_height_operation(2));

            batch.push(update_unpaid_epoch_index_operation(1));

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let unpaid_epoch = platform
                .find_oldest_epoch_needing_payment(current_epoch_index, None, Some(&transaction))
                .expect("should find nothing");

            assert!(unpaid_epoch.is_none());
        }

        #[test]
        fn test_use_cached_current_start_block_height_as_end_block_if_unpaid_epoch_is_previous() {
            let platform = setup_platform_with_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let epoch_0_tree = Epoch::new(GENESIS_EPOCH_INDEX);

            let current_epoch_index = GENESIS_EPOCH_INDEX + 1;

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch_0_tree.update_start_block_height_operation(1));

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let cached_current_epoch_start_block_height = Some(2);

            let unpaid_epoch = platform
                .find_oldest_epoch_needing_payment(
                    current_epoch_index,
                    cached_current_epoch_start_block_height,
                    Some(&transaction),
                )
                .expect("should find nothing");

            match unpaid_epoch {
                Some(unpaid_epoch) => {
                    assert_eq!(unpaid_epoch.epoch_index, 0);
                    assert_eq!(unpaid_epoch.next_unpaid_epoch_index, 1);
                    assert_eq!(unpaid_epoch.start_block_height, 1);
                    assert_eq!(unpaid_epoch.end_block_height, 2);

                    let block_count = unpaid_epoch.block_count().expect("should calculate ");

                    assert_eq!(block_count, 1);
                }
                None => assert!(false, "unpaid epoch should be present"),
            }
        }

        #[test]
        fn test_use_stored_start_block_height_from_current_epoch_as_end_block_if_unpaid_epoch_is_previous(
        ) {
            let platform = setup_platform_with_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let epoch_0_tree = Epoch::new(GENESIS_EPOCH_INDEX);

            let current_epoch_index = GENESIS_EPOCH_INDEX + 1;

            let epoch_1_tree = Epoch::new(current_epoch_index);

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch_0_tree.update_start_block_height_operation(1));
            batch.push(epoch_1_tree.update_start_block_height_operation(2));

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let unpaid_epoch = platform
                .find_oldest_epoch_needing_payment(current_epoch_index, None, Some(&transaction))
                .expect("should find nothing");

            match unpaid_epoch {
                Some(unpaid_epoch) => {
                    assert_eq!(unpaid_epoch.epoch_index, 0);
                    assert_eq!(unpaid_epoch.next_unpaid_epoch_index, 1);
                    assert_eq!(unpaid_epoch.start_block_height, 1);
                    assert_eq!(unpaid_epoch.end_block_height, 2);

                    let block_count = unpaid_epoch.block_count().expect("should calculate ");

                    assert_eq!(block_count, 1);
                }
                None => assert!(false, "unpaid epoch should be present"),
            }
        }

        #[test]
        fn test_find_stored_next_start_block_as_end_block_if_unpaid_epoch_more_than_one_ago() {
            let platform = setup_platform_with_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let epoch_0_tree = Epoch::new(GENESIS_EPOCH_INDEX);
            let epoch_1_tree = Epoch::new(GENESIS_EPOCH_INDEX + 1);

            let current_epoch_index = GENESIS_EPOCH_INDEX + 2;

            let epoch_2_tree = Epoch::new(current_epoch_index);

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch_0_tree.update_start_block_height_operation(1));
            batch.push(epoch_1_tree.update_start_block_height_operation(2));
            batch.push(epoch_2_tree.update_start_block_height_operation(3));

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let unpaid_epoch = platform
                .find_oldest_epoch_needing_payment(current_epoch_index, None, Some(&transaction))
                .expect("should find nothing");

            match unpaid_epoch {
                Some(unpaid_epoch) => {
                    assert_eq!(unpaid_epoch.epoch_index, 0);
                    assert_eq!(unpaid_epoch.next_unpaid_epoch_index, 1);
                    assert_eq!(unpaid_epoch.start_block_height, 1);
                    assert_eq!(unpaid_epoch.end_block_height, 2);

                    let block_count = unpaid_epoch.block_count().expect("should calculate ");

                    assert_eq!(block_count, 1);
                }
                None => assert!(false, "unpaid epoch should be present"),
            }
        }

        #[test]
        fn test_use_cached_start_block_height_if_not_found_in_case_of_epoch_change() {
            let platform = setup_platform_with_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let epoch_0_tree = Epoch::new(GENESIS_EPOCH_INDEX);

            let current_epoch_index = GENESIS_EPOCH_INDEX + 2;

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch_0_tree.update_start_block_height_operation(1));

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let cached_current_epoch_start_block_height = Some(2);

            let unpaid_epoch = platform
                .find_oldest_epoch_needing_payment(
                    current_epoch_index,
                    cached_current_epoch_start_block_height,
                    Some(&transaction),
                )
                .expect("should find nothing");

            match unpaid_epoch {
                Some(unpaid_epoch) => {
                    assert_eq!(unpaid_epoch.epoch_index, 0);
                    assert_eq!(unpaid_epoch.next_unpaid_epoch_index, 2);
                    assert_eq!(unpaid_epoch.start_block_height, 1);
                    assert_eq!(unpaid_epoch.end_block_height, 2);

                    let block_count = unpaid_epoch.block_count().expect("should calculate ");

                    assert_eq!(block_count, 1);
                }
                None => assert!(false, "unpaid epoch should be present"),
            }
        }

        #[test]
        fn test_error_if_cached_start_block_height_is_not_present_and_not_found_in_case_of_epoch_change(
        ) {
            let platform = setup_platform_with_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let epoch_0_tree = Epoch::new(GENESIS_EPOCH_INDEX);

            let current_epoch_index = GENESIS_EPOCH_INDEX + 2;

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch_0_tree.update_start_block_height_operation(1));

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let unpaid_epoch = platform.find_oldest_epoch_needing_payment(
                current_epoch_index,
                None,
                Some(&transaction),
            );

            match unpaid_epoch {
                Ok(_) => assert!(false, "should return code execution error"),
                Err(Error::Execution(ExecutionError::CorruptedCodeExecution(_))) => assert!(true),
                Err(_) => assert!(false, "wrong error type"),
            }
        }
    }

    mod add_epoch_pool_to_proposers_payout_operations {
        use crate::common::helpers::fee_pools::{
            create_test_masternode_share_identities_and_documents, refetch_identities,
        };
        use crate::common::helpers::setup::setup_platform_with_initial_state_structure;
        use crate::execution::fee_pools::fee_distribution::UnpaidEpoch;
        use rs_drive::common::helpers::identities::create_test_masternode_identities_and_add_them_as_epoch_block_proposers;
        use rs_drive::drive::batch::GroveDbOpBatch;
        use rs_drive::fee_pools::epochs::Epoch;
        use rust_decimal::Decimal;
        use rust_decimal_macros::dec;

        #[test]
        fn test_payout_to_proposers() {
            let platform = setup_platform_with_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            // Create masternode reward shares contract
            let contract = platform.create_mn_shares_contract(Some(&transaction));

            let proposers_count = 10u16;
            let processing_fees = 10000;
            let storage_fees = 10000;

            let unpaid_epoch_tree = Epoch::new(0);
            let next_epoch_tree = Epoch::new(1);

            let mut batch = GroveDbOpBatch::new();

            unpaid_epoch_tree.add_init_current_operations(1.0, 1, 1, &mut batch);

            batch.push(
                unpaid_epoch_tree
                    .update_processing_credits_for_distribution_operation(processing_fees),
            );

            batch.push(
                unpaid_epoch_tree.update_storage_credits_for_distribution_operation(storage_fees),
            );

            next_epoch_tree.add_init_current_operations(
                1.0,
                proposers_count as u64 + 1,
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
                    Some(&transaction),
                );

            let share_identities_and_documents =
                create_test_masternode_share_identities_and_documents(
                    &platform.drive,
                    &contract,
                    &pro_tx_hashes,
                    Some(&transaction),
                );

            let mut batch = GroveDbOpBatch::new();

            let unpaid_epoch = UnpaidEpoch {
                epoch_index: 0,
                start_block_height: 1,
                end_block_height: 11,
                next_unpaid_epoch_index: 0,
            };

            let proposers_paid_count = platform
                .add_epoch_pool_to_proposers_payout_operations(
                    &unpaid_epoch,
                    proposers_count,
                    Some(&transaction),
                    &mut batch,
                )
                .expect("should distribute fees");

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            assert_eq!(proposers_paid_count, 10);

            // check we paid 500 to every mn identity
            let paid_mn_identities = platform
                .drive
                .fetch_identities(&pro_tx_hashes, Some(&transaction))
                .expect("expected to get identities");

            let total_fees = Decimal::from(storage_fees + processing_fees);

            let masternode_reward = total_fees / Decimal::from(proposers_count);

            let shares_percentage_with_precision: u64 = share_identities_and_documents[0]
                .1
                .properties
                .get("percentage")
                .expect("should have percentage field")
                .as_integer()
                .expect("percentage should an integer")
                .try_into()
                .expect("percentage should be u64");

            let shares_percentage = Decimal::from(shares_percentage_with_precision) / dec!(10000);

            let payout_credits = masternode_reward * shares_percentage;

            let payout_credits: u64 = payout_credits.try_into().expect("should convert to i64");

            for paid_mn_identity in paid_mn_identities {
                assert_eq!(paid_mn_identity.balance, payout_credits);
            }

            let share_identities = share_identities_and_documents
                .iter()
                .map(|(identity, _)| identity)
                .collect();

            let refetched_share_identities =
                refetch_identities(&platform.drive, share_identities, Some(&transaction))
                    .expect("expected to refresh identities");

            for identity in refetched_share_identities {
                assert_eq!(identity.balance, payout_credits);
            }
        }
    }

    mod add_distribute_block_fees_into_pools_operations {
        use crate::abci::messages::FeesAggregate;
        use crate::common::helpers::setup::setup_platform_with_initial_state_structure;
        use rs_drive::drive::batch::GroveDbOpBatch;
        use rs_drive::fee_pools::epochs::Epoch;

        #[test]
        fn test_distribute_block_fees_into_uncommitted_epoch_on_epoch_change() {
            let platform = setup_platform_with_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let current_epoch_tree = Epoch::new(1);

            let mut batch = GroveDbOpBatch::new();

            current_epoch_tree.add_init_current_operations(1.0, 1, 1, &mut batch);

            let processing_fees = 1000000;
            let storage_fees = 2000000;

            platform
                .add_distribute_block_fees_into_pools_operations(
                    &current_epoch_tree,
                    &FeesAggregate {
                        processing_fees,
                        storage_fees,
                    },
                    None,
                    Some(&transaction),
                    &mut batch,
                )
                .expect("should distribute fees into pools");

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
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
                .get_aggregate_storage_fees_from_distribution_pool(Some(&transaction))
                .expect("should get storage fee pool");

            assert_eq!(stored_processing_fee_credits, processing_fees);
            assert_eq!(stored_storage_fee_credits, storage_fees);
        }

        #[test]
        fn test_distribute_block_fees_into_pools() {
            let platform = setup_platform_with_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let current_epoch_tree = Epoch::new(1);

            let mut batch = GroveDbOpBatch::new();

            current_epoch_tree.add_init_current_operations(1.0, 1, 1, &mut batch);

            // Apply new pool structure
            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let mut batch = GroveDbOpBatch::new();

            let processing_fees = 1000000;
            let storage_fees = 2000000;

            platform
                .add_distribute_block_fees_into_pools_operations(
                    &current_epoch_tree,
                    &FeesAggregate {
                        processing_fees,
                        storage_fees,
                    },
                    None,
                    Some(&transaction),
                    &mut batch,
                )
                .expect("should distribute fees into pools");

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
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
                .get_aggregate_storage_fees_from_distribution_pool(Some(&transaction))
                .expect("should get storage fee pool");

            assert_eq!(stored_processing_fee_credits, processing_fees);
            assert_eq!(stored_storage_fee_credits, storage_fees);
        }
    }
}
