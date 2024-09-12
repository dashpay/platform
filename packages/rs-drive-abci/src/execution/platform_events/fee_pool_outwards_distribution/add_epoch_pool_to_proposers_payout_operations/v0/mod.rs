use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::execution::types::unpaid_epoch::v0::{UnpaidEpochV0Getters, UnpaidEpochV0Methods};
use crate::execution::types::unpaid_epoch::UnpaidEpoch;
use crate::platform_types::platform::Platform;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::document::DocumentV0Getters;
use dpp::fee::Credits;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;

use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::query::proposer_block_count_query::ProposerQueryType;
use drive::util::batch::DriveOperation;
use drive::util::batch::DriveOperation::IdentityOperation;
use drive::util::batch::IdentityOperationType::AddToIdentityBalance;

use drive::grovedb::Transaction;

impl<C> Platform<C> {
    /// Adds operations to the op batch which distribute the fees from an unpaid epoch pool
    /// to the total fees to be paid out to proposers and divides amongst masternode reward shares.
    ///
    /// Returns the number of proposers to be paid out.
    pub(super) fn add_epoch_pool_to_proposers_payout_operations_v0(
        &self,
        unpaid_epoch: &UnpaidEpoch,
        core_block_rewards: Credits,
        transaction: &Transaction,
        batch: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<u16, Error> {
        let mut drive_operations = vec![];
        let unpaid_epoch_tree = Epoch::new(unpaid_epoch.epoch_index())?;

        let storage_and_processing_fees = self
            .drive
            .get_epoch_total_credits_for_distribution(
                &unpaid_epoch_tree,
                Some(transaction),
                platform_version,
            )
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
            .fetch_epoch_proposers(
                &unpaid_epoch_tree,
                ProposerQueryType::ByRange(None, None),
                Some(transaction),
                platform_version,
            )
            .map_err(Error::Drive)?;

        let proposers_len = proposers.len() as u16;

        tracing::trace!(
            unpaid_block_count = unpaid_epoch_block_count,
            unpaid_epoch_index = unpaid_epoch.epoch_index(),
            fees = storage_and_processing_fees,
            core_block_rewards,
            total_payouts,
            "Pay {total_payouts} credits to {proposers_len} proposers for {} proposed blocks in epoch {}",
            unpaid_epoch_block_count,
            unpaid_epoch.epoch_index(),
        );

        for (i, (proposer_tx_hash, proposed_block_count)) in proposers.into_iter().enumerate() {
            let i = i as u16;

            let total_masternode_payout = total_payouts
                .checked_mul(proposed_block_count)
                .and_then(|r| r.checked_div(unpaid_epoch_block_count))
                .ok_or(Error::Execution(ExecutionError::Overflow(
                    "overflow when getting masternode reward division",
                )))?;

            let mut masternode_payout_leftover = total_masternode_payout;

            let documents = self.fetch_reward_shares_list_for_masternode(
                &proposer_tx_hash,
                Some(transaction),
                platform_version,
            )?;

            for document in documents {
                let pay_to_id = document
                    .properties()
                    .get_identifier("payToId")
                    .map_err(|e| Error::Protocol(ProtocolError::ValueError(e)))?;

                // TODO this shouldn't be a percentage we need to update masternode share contract
                let share_percentage: u64 = document
                    .properties()
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
            platform_version,
        )?;

        batch.push(DriveOperation::GroveDBOpBatch(operations));

        Ok(proposers_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

    mod add_epoch_pool_to_proposers_payout_operations {
        use super::*;
        use crate::execution::types::unpaid_epoch::v0::UnpaidEpochV0;
        use crate::test::helpers::{
            fee_pools::create_test_masternode_share_identities_and_documents,
            setup::TestPlatformBuilder,
        };
        use dpp::block::block_info::BlockInfo;
        use dpp::identity::accessors::IdentityGettersV0;
        use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;

        use drive::drive::credit_pools::epochs::operations_factory::EpochOperations;
        use drive::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
        use drive::util::batch::GroveDbOpBatch;
        use drive::util::test_helpers::test_utils::identities::create_test_masternode_identities_and_add_them_as_epoch_block_proposers;
        use rust_decimal::Decimal;
        use rust_decimal_macros::dec;

        #[test]
        fn test_payout_to_proposers() {
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let platform_read_guard = platform.state.load();
            let platform_version = platform_read_guard
                .current_platform_version()
                .expect("platform_version");
            let transaction = platform.drive.grove.start_transaction();

            // Create masternode reward shares contract
            let contract = platform.create_mn_shares_contract(Some(&transaction), platform_version);

            let proposers_count = 10u16;
            let processing_fees = 10000;
            let storage_fees = 10000;

            let unpaid_epoch_tree = Epoch::new(0).unwrap();
            let next_epoch_tree = Epoch::new(1).unwrap();

            let mut batch = GroveDbOpBatch::new();

            unpaid_epoch_tree.add_init_current_operations(
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
                platform_version
                    .fee_version
                    .uses_version_fee_multiplier_permille
                    .expect("expected a fee multiplier"),
                proposers_count as u64 + 1,
                1,
                10,
                platform_version.protocol_version,
                &mut batch,
            );

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let pro_tx_hashes =
                create_test_masternode_identities_and_add_them_as_epoch_block_proposers(
                    &platform.drive,
                    &unpaid_epoch_tree,
                    proposers_count,
                    Some(68), //random number
                    Some(&transaction),
                    platform_version,
                );

            let share_identities_and_documents =
                create_test_masternode_share_identities_and_documents(
                    &platform.drive,
                    &contract,
                    &pro_tx_hashes,
                    Some(55),
                    Some(&transaction),
                    platform_version,
                );

            let mut batch = vec![];

            let unpaid_epoch = UnpaidEpochV0 {
                epoch_index: 0,
                start_block_height: 1,
                next_epoch_start_block_height: 11,
                start_block_core_height: 1,
                next_unpaid_epoch_index: 0,
                next_epoch_start_block_core_height: 1,
            };

            let proposers_paid_count = platform
                .add_epoch_pool_to_proposers_payout_operations_v0(
                    &unpaid_epoch.into(),
                    0,
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

            assert_eq!(proposers_paid_count, 10);

            // check we paid 500 to every mn identity
            let paid_mn_identities_balances = platform
                .drive
                .fetch_identities_balances(&pro_tx_hashes, Some(&transaction), platform_version)
                .expect("expected to get identities");

            let total_fees = Decimal::from(storage_fees + processing_fees);

            let masternode_reward = total_fees / Decimal::from(proposers_count);

            let shares_percentage_with_precision: u64 = share_identities_and_documents[0]
                .1
                .properties()
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
                .map(|(identity, _)| identity.id().to_buffer())
                .collect();

            let refetched_share_identities_balances = platform
                .drive
                .fetch_identities_balances(&share_identities, Some(&transaction), platform_version)
                .expect("expected to get identities");

            for (_, balance) in refetched_share_identities_balances {
                assert_eq!(balance, payout_credits);
            }
        }
    }
}
