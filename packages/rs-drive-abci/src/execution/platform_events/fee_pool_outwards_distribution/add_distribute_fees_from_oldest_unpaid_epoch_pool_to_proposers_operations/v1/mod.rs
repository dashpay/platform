use crate::error::Error;
use crate::execution::types::proposer_payouts;
use crate::platform_types::platform::Platform;
use dpp::block::epoch::Epoch;
use dpp::block::finalized_epoch_info::v0::FinalizedEpochInfoV0;
use dpp::block::finalized_epoch_info::FinalizedEpochInfo;
use dpp::core_subsidy::epoch_core_reward_credits_for_distribution::epoch_core_reward_credits_for_distribution;
use dpp::core_subsidy::NetworkCoreSubsidy;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use drive::drive::credit_pools::epochs::operations_factory::EpochOperations;
use drive::drive::credit_pools::operations::update_unpaid_epoch_index_operation;
use drive::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use drive::util::batch::{DriveOperation, GroveDbOpBatch, SystemOperationType};

use crate::execution::types::unpaid_epoch::v0::{UnpaidEpochV0Getters, UnpaidEpochV0Methods};

use drive::grovedb::Transaction;

impl<C> Platform<C> {
    /// Adds operations to the op batch which distribute fees
    /// from the oldest unpaid epoch pool to proposers.
    ///
    /// Returns `ProposersPayouts` if there are any.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations_v1(
        &self,
        current_epoch_index: u16,
        cached_current_epoch_start_block_height: Option<u64>,
        cached_current_epoch_start_block_core_height: Option<u32>,
        total_distributed_storage_fees: Credits,
        transaction: &Transaction,
        batch: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<proposer_payouts::v0::ProposersPayouts>, Error> {
        let unpaid_epoch = self.find_oldest_epoch_needing_payment(
            current_epoch_index,
            cached_current_epoch_start_block_height,
            cached_current_epoch_start_block_core_height,
            Some(transaction),
            platform_version,
        )?;

        let Some(unpaid_epoch) = unpaid_epoch else {
            return Ok(None);
        };

        let start_block_core_height = if unpaid_epoch.epoch_index == 0 {
            //On epoch 0 we need to use the activation height instead of the start of the block
            self.drive
                .fetch_genesis_core_height(Some(transaction), platform_version)?
        } else {
            // The unpaid epochs start block has had its credits distributed, so we must do a + 1
            // But only if we are not in the first epoch
            unpaid_epoch.start_block_core_height + 1
        };

        // Calculate core block reward for the unpaid epoch
        let core_block_rewards = epoch_core_reward_credits_for_distribution(
            start_block_core_height,
            unpaid_epoch.next_epoch_start_block_core_height,
            self.config.network.core_subsidy_halving_interval(),
            platform_version,
        )?;

        // We must add to the system credits the epoch core block rewards
        // On the Core side we move block rewards every block to asset lock pool
        batch.push(DriveOperation::SystemOperation(
            SystemOperationType::AddToSystemCredits {
                amount: core_block_rewards,
            },
        ));

        tracing::info!(
            "Core block rewards for epoch {} from height {} to height {} are {}",
            unpaid_epoch.epoch_index,
            unpaid_epoch.start_block_core_height,
            unpaid_epoch.next_epoch_start_block_core_height,
            core_block_rewards
        );

        let unpaid_epoch = unpaid_epoch.into();

        let (storage_and_processing_credits, block_proposers) = self
            .add_epoch_pool_to_proposers_payout_operations(
                &unpaid_epoch,
                core_block_rewards,
                transaction,
                batch,
                platform_version,
            )?;

        let mut inner_batch = GroveDbOpBatch::new();

        let unpaid_epoch_tree = Epoch::new(unpaid_epoch.epoch_index())?;

        unpaid_epoch_tree.add_mark_as_paid_operations(&mut inner_batch);

        inner_batch.push(update_unpaid_epoch_index_operation(
            unpaid_epoch.next_unpaid_epoch_index(),
        ));

        let proposers_paid_count = block_proposers.len() as u16;

        let finalized_epoch_info: FinalizedEpochInfo = FinalizedEpochInfoV0 {
            first_block_time: unpaid_epoch.epoch_start_time(),
            first_block_height: unpaid_epoch.start_block_height(),
            total_blocks_in_epoch: unpaid_epoch.block_count()?,
            first_core_block_height: start_block_core_height,
            next_epoch_start_core_block_height: unpaid_epoch.next_epoch_start_block_core_height(),
            total_processing_fees: storage_and_processing_credits.processing_pool_credits,
            total_distributed_storage_fees: storage_and_processing_credits.storage_pool_credits,
            total_created_storage_fees: total_distributed_storage_fees,
            core_block_rewards,
            block_proposers,
            fee_multiplier_permille: unpaid_epoch.fee_multiplier(),
            protocol_version: unpaid_epoch.protocol_version(),
        }
        .into();

        let add_epoch_final_info_operation = self.drive.add_epoch_final_info_operation(
            &unpaid_epoch_tree,
            finalized_epoch_info,
            platform_version,
        )?;

        inner_batch.push(add_epoch_final_info_operation);

        batch.push(DriveOperation::GroveDBOpBatch(inner_batch));

        // We paid to all epoch proposers last block. Since proposers paid count
        // was equal to proposers limit, we paid to 0 proposers this block
        if proposers_paid_count == 0 {
            return Ok(None);
        }

        Ok(Some(proposer_payouts::v0::ProposersPayouts {
            proposers_paid_count,
            paid_epoch_index: unpaid_epoch.epoch_index(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::block::block_info::BlockInfo;

    use drive::util::test_helpers::test_utils::identities::create_test_masternode_identities_and_add_them_as_epoch_block_proposers;

    use crate::test::helpers::setup::TestPlatformBuilder;

    use crate::execution::types::proposer_payouts::v0::ProposersPayouts;
    use drive::error::Error as DriveError;
    use drive::grovedb;

    #[test]
    fn test_nothing_to_distribute_if_there_is_no_epochs_needing_payment() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let transaction = platform.drive.grove.start_transaction();

        let current_epoch_index = 0;

        let mut batch = vec![];

        let proposers_payouts = platform
            .add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations_v0(
                current_epoch_index,
                None,
                None,
                &transaction,
                &mut batch,
                platform_version,
            )
            .expect("should distribute fees");

        assert!(proposers_payouts.is_none());
    }

    #[test]
    fn test_mark_epoch_as_paid_and_update_next_update_epoch_index_if_all_proposers_paid() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state_with_activation_info(0, 1);
        let transaction = platform.drive.grove.start_transaction();

        // Create masternode reward shares contract
        platform.create_mn_shares_contract(Some(&transaction), platform_version);

        let proposers_count = 150;
        let processing_fees = 100000000;
        let storage_fees = 10000000;

        let unpaid_epoch = Epoch::new(0).unwrap();
        let current_epoch = Epoch::new(1).unwrap();

        let mut batch = GroveDbOpBatch::new();

        unpaid_epoch.add_init_current_operations(
            platform_version
                .fee_version
                .uses_version_fee_multiplier_permille
                .expect("expected a fee multiplier"),
            1,
            1,
            1,
            platform_version.protocol_version,
            &mut batch,
        );

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
            platform_version
                .fee_version
                .uses_version_fee_multiplier_permille
                .expect("expected a fee multiplier"),
            proposers_count as u64 + 1,
            3,
            2,
            platform_version.protocol_version,
            &mut batch,
        );

        platform
            .drive
            .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
            .expect("should apply batch");

        let proposers = create_test_masternode_identities_and_add_them_as_epoch_block_proposers(
            &platform.drive,
            &unpaid_epoch,
            proposers_count,
            Some(65), //random number
            Some(&transaction),
            platform_version,
        );

        let mut batch = vec![];

        let proposer_payouts = platform
            .add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations_v0(
                current_epoch.index,
                None,
                None,
                &transaction,
                &mut batch,
                platform_version,
            )
            .expect("should distribute fees");

        platform
            .drive
            .apply_drive_operations(
                batch,
                true,
                &BlockInfo::default(),
                Some(&transaction),
                platform_version,
                None,
            )
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
            .get_unpaid_epoch_index(Some(&transaction), platform_version)
            .expect("should get unpaid epoch index");

        assert_eq!(next_unpaid_epoch_index, current_epoch.index);

        // check we've removed proposers tree
        let result = platform.drive.get_epochs_proposer_block_count(
            &unpaid_epoch,
            &proposers[0],
            Some(&transaction),
            platform_version,
        );

        assert!(matches!(
            result,
            Err(DriveError::GroveDB(
                grovedb::Error::PathParentLayerNotFound(_)
            ))
        ));
    }
}
