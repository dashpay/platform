use crate::error::execution::ExecutionError;
use crate::error::Error;
use std::collections::BTreeMap;

use crate::execution::types::unpaid_epoch::v0::{UnpaidEpochV0Getters, UnpaidEpochV0Methods};
use crate::execution::types::unpaid_epoch::UnpaidEpoch;
use crate::platform_types::platform::Platform;
use dpp::block::epoch::Epoch;
use dpp::block::pool_credits::StorageAndProcessingPoolCredits;
use dpp::document::DocumentV0Getters;
use dpp::fee::Credits;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::platform_value::Identifier;
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
    ///
    /// Generation 1 (protocol version 14) is generation 0, and the payouts join the batch as
    /// the identity credits they are instead of grove operations converted here: the block's
    /// `apply_drive_operations` converts them against the same state and routes the credits
    /// that repay a recipient's debt to the processing fee pool of the block's epoch, after the
    /// batch's own fee distribution. A plain grove batch cannot carry them, and generation 0,
    /// which converted with `convert_drive_operations_to_grove_operations`, left them in no
    /// balance the credit sum counts. A share whose `payToId` has no balance is skipped and
    /// stays with its masternode, and each share is capped at what is left of the masternode's
    /// payout, so neither fails the batch that ends the block.
    pub(super) fn add_epoch_pool_to_proposers_payout_operations_v1(
        &self,
        unpaid_epoch: &UnpaidEpoch,
        core_block_rewards: Credits,
        transaction: &Transaction,
        batch: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(StorageAndProcessingPoolCredits, BTreeMap<Identifier, u64>), Error> {
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
            .total_credits
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
            core_block_rewards,
            total_payouts,
            "Pay {total_payouts} credits to {proposers_len} proposers for {} proposed blocks in epoch {}, decomposed as {}",
            unpaid_epoch_block_count,
            unpaid_epoch.epoch_index(),
            storage_and_processing_fees
        );

        for (i, (proposer_tx_hash, proposed_block_count)) in proposers.iter().enumerate() {
            let i = i as u16;

            let total_masternode_payout = total_payouts
                .checked_mul(*proposed_block_count)
                .and_then(|r| r.checked_div(unpaid_epoch_block_count))
                .ok_or(Error::Execution(ExecutionError::Overflow(
                    "overflow when getting masternode reward division",
                )))?;

            let mut masternode_payout_leftover = total_masternode_payout;

            let documents = self.fetch_reward_shares_list_for_masternode(
                proposer_tx_hash.as_bytes(),
                Some(transaction),
                platform_version,
            )?;

            for document in documents {
                let pay_to_id = document
                    .properties()
                    .get_identifier("payToId")
                    .map_err(|e| Error::Protocol(ProtocolError::ValueError(e)))?;

                // A share naming an identity that has no balance stays with its masternode:
                // crediting it would fail the batch that ends the block
                if self
                    .drive
                    .fetch_identity_balance(
                        pay_to_id.to_buffer(),
                        Some(transaction),
                        platform_version,
                    )?
                    .is_none()
                {
                    continue;
                }

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

                // Shares above 100% in total get what is left, not a failed payout
                let share_payout = total_masternode_payout
                    .checked_mul(share_percentage)
                    .and_then(|a| a.checked_div(10000))
                    .ok_or(Error::Execution(ExecutionError::Overflow(
                        "overflow when calculating reward share",
                    )))?
                    .min(masternode_payout_leftover);

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

            drive_operations.push(IdentityOperation(AddToIdentityBalance {
                identity_id: proposer_tx_hash.to_buffer(),
                added_balance: proposer_payout,
            }));
        }

        batch.extend(drive_operations);

        Ok((storage_and_processing_fees, proposers.into_iter().collect()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_types::platform_state::PlatformStateV0Methods;

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

        use crate::rpc::core::MockCoreRPCLike;
        use crate::test::helpers::setup::TempPlatform;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::data_contract::DataContract;
        use dpp::document::{Document, DocumentV0, INITIAL_REVISION};
        use dpp::platform_value::Value;
        use dpp::system_data_contracts::masternode_reward_shares_contract::v1::document_types;
        use drive::drive::credit_pools::epochs::operations_factory::EpochOperations;
        use drive::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
        use drive::util::batch::GroveDbOpBatch;
        use drive::util::object_size_info::DocumentInfo::DocumentRefInfo;
        use drive::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
        use drive::util::storage_flags::StorageFlags;
        use drive::util::test_helpers::test_utils::identities::{
            create_test_identity,
            create_test_masternode_identities_and_add_them_as_epoch_block_proposers,
        };
        use rust_decimal::Decimal;
        use rust_decimal_macros::dec;
        use std::borrow::Cow;

        #[test]
        fn should_pay_proposers_and_shares_and_credit_a_repaid_debt_to_the_block_epochs_pool() {
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

            // The first share recipient owes 7 credits, the unpaid part of an earlier fee
            let indebted_share_identity = share_identities_and_documents[0].0.id().to_buffer();
            let owed = 7;
            let debt_operation = platform
                .drive
                .update_identity_negative_credit_operation(
                    indebted_share_identity,
                    owed,
                    platform_version,
                )
                .expect("expected a debt operation");
            platform
                .drive
                .apply_batch_low_level_drive_operations(
                    None,
                    Some(&transaction),
                    vec![debt_operation],
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected to store the debt");

            let mut batch = vec![];

            let unpaid_epoch = UnpaidEpochV0 {
                epoch_index: 0,
                start_block_height: 1,
                next_epoch_start_block_height: 11,
                start_block_core_height: 1,
                next_unpaid_epoch_index: 0,
                next_epoch_start_block_core_height: 1,
                epoch_start_time: 0,
                protocol_version: platform_version.protocol_version,
                fee_multiplier: 0,
            };

            let proposers_paid_count = platform
                .add_epoch_pool_to_proposers_payout_operations_v1(
                    &unpaid_epoch.into(),
                    0,
                    &transaction,
                    &mut batch,
                    platform_version,
                )
                .expect("should distribute fees")
                .1;

            // The payout is applied in a block of the next epoch
            platform
                .drive
                .apply_drive_operations(
                    batch,
                    true,
                    &BlockInfo::default_with_epoch(next_epoch_tree),
                    Some(&transaction),
                    platform_version,
                    None,
                )
                .expect("should apply batch");

            assert_eq!(proposers_paid_count.len(), 10);

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

            for (identity_id, balance) in refetched_share_identities_balances {
                if identity_id == indebted_share_identity {
                    assert_eq!(balance, payout_credits - owed);
                } else {
                    assert_eq!(balance, payout_credits);
                }
            }

            // The repaid part reached the processing fee pool of the block's epoch
            let next_epoch_processing_fees = platform
                .drive
                .get_epoch_processing_credits_for_distribution(
                    &next_epoch_tree,
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected the next epoch's processing fees");
            assert_eq!(next_epoch_processing_fees, owed);
        }

        /// Writes a reward share of `masternode` paying `percentage` (of 10000) to `pay_to`
        fn insert_reward_share(
            platform: &TempPlatform<MockCoreRPCLike>,
            contract: &DataContract,
            masternode: [u8; 32],
            pay_to: [u8; 32],
            percentage: u16,
            transaction: &Transaction,
            platform_version: &PlatformVersion,
        ) {
            let document: Document = DocumentV0 {
                id: Identifier::random(),
                owner_id: Identifier::new(masternode),
                properties: BTreeMap::from([
                    ("payToId".to_string(), Value::Bytes(pay_to.to_vec())),
                    ("percentage".to_string(), percentage.into()),
                ]),
                revision: Some(INITIAL_REVISION),
                ..Default::default()
            }
            .into();
            let document_type = contract
                .document_type_for_name(document_types::reward_share::NAME)
                .expect("expected the reward share document type");
            platform
                .drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((
                                &document,
                                Some(Cow::Owned(StorageFlags::SingleEpoch(0))),
                            )),
                            owner_id: None,
                        },
                        contract,
                        document_type,
                    },
                    false,
                    BlockInfo::genesis(),
                    true,
                    Some(transaction),
                    platform_version,
                    None,
                )
                .expect("expected to insert the reward share");
        }

        #[test]
        fn should_repay_a_shared_recipients_debt_once_from_every_share_one_payout_pays_it() {
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let platform_read_guard = platform.state.load();
            let platform_version = platform_read_guard
                .current_platform_version()
                .expect("platform_version");
            let transaction = platform.drive.grove.start_transaction();

            let contract = platform.create_mn_shares_contract(Some(&transaction), platform_version);

            let proposers_count = 2u16;
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
                    Some(68),
                    Some(&transaction),
                    platform_version,
                );

            // Both masternodes pay half their reward to the same recipient, who owes 7 credits.
            // A masternode's payout reads one reward share, so the recipient is paid once by each
            let shared_recipient = create_test_identity(
                &platform.drive,
                [7; 32],
                Some(7),
                Some(&transaction),
                platform_version,
            )
            .expect("expected the shared recipient")
            .id()
            .to_buffer();
            for masternode in &pro_tx_hashes {
                insert_reward_share(
                    &platform,
                    &contract,
                    *masternode,
                    shared_recipient,
                    5000,
                    &transaction,
                    platform_version,
                );
            }
            let owed = 7;
            let debt_operation = platform
                .drive
                .update_identity_negative_credit_operation(shared_recipient, owed, platform_version)
                .expect("expected a debt operation");
            platform
                .drive
                .apply_batch_low_level_drive_operations(
                    None,
                    Some(&transaction),
                    vec![debt_operation],
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected to store the debt");

            let unpaid_epoch = UnpaidEpochV0 {
                epoch_index: 0,
                start_block_height: 1,
                // One block per proposer, so each masternode's reward is its share of the epoch
                next_epoch_start_block_height: 1 + proposers_count as u64,
                start_block_core_height: 1,
                next_unpaid_epoch_index: 0,
                next_epoch_start_block_core_height: 1,
                epoch_start_time: 0,
                protocol_version: platform_version.protocol_version,
                fee_multiplier: 0,
            };

            let mut batch = vec![];
            platform
                .add_epoch_pool_to_proposers_payout_operations_v1(
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
                    &BlockInfo::default_with_epoch(next_epoch_tree),
                    Some(&transaction),
                    platform_version,
                    None,
                )
                .expect("should apply batch");

            // Each masternode proposed one of the epoch's two blocks, so its reward is half the
            // pools, and its share half of that
            let share = (storage_fees + processing_fees) / proposers_count as u64 / 2;
            // Both shares landed and the debt was repaid once: the balance holds the rest, and
            // the balance less the debt is the same number, so no debt is left
            let shared_recipient_balance = platform
                .drive
                .fetch_identity_balance(shared_recipient, Some(&transaction), platform_version)
                .expect("expected the balance")
                .expect("expected the identity");
            assert_eq!(shared_recipient_balance, 2 * share - owed);
            let shared_recipient_balance_less_debt = platform
                .drive
                .fetch_identity_balance_include_debt(
                    shared_recipient,
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected the balance")
                .expect("expected the identity");
            assert_eq!(
                shared_recipient_balance_less_debt,
                (2 * share - owed) as i64
            );

            let next_epoch_processing_fees = platform
                .drive
                .get_epoch_processing_credits_for_distribution(
                    &next_epoch_tree,
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected the next epoch's processing fees");
            assert_eq!(next_epoch_processing_fees, owed);
        }

        #[test]
        fn should_keep_a_share_without_a_recipient_and_cap_a_share_above_the_reward() {
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let platform_read_guard = platform.state.load();
            let platform_version = platform_read_guard
                .current_platform_version()
                .expect("platform_version");
            let transaction = platform.drive.grove.start_transaction();

            let contract = platform.create_mn_shares_contract(Some(&transaction), platform_version);

            let proposers_count = 2u16;
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
                    Some(68),
                    Some(&transaction),
                    platform_version,
                );

            // The first masternode's share names no identity; the second's claims 120% of its
            // reward for one recipient
            insert_reward_share(
                &platform,
                &contract,
                pro_tx_hashes[0],
                [9; 32],
                5000,
                &transaction,
                platform_version,
            );
            let recipient = create_test_identity(
                &platform.drive,
                [7; 32],
                Some(7),
                Some(&transaction),
                platform_version,
            )
            .expect("expected the recipient")
            .id()
            .to_buffer();
            insert_reward_share(
                &platform,
                &contract,
                pro_tx_hashes[1],
                recipient,
                12000,
                &transaction,
                platform_version,
            );

            let unpaid_epoch = UnpaidEpochV0 {
                epoch_index: 0,
                start_block_height: 1,
                next_epoch_start_block_height: 1 + proposers_count as u64,
                start_block_core_height: 1,
                next_unpaid_epoch_index: 0,
                next_epoch_start_block_core_height: 1,
                epoch_start_time: 0,
                protocol_version: platform_version.protocol_version,
                fee_multiplier: 0,
            };

            let mut batch = vec![];
            platform
                .add_epoch_pool_to_proposers_payout_operations_v1(
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
                    &BlockInfo::default_with_epoch(next_epoch_tree),
                    Some(&transaction),
                    platform_version,
                    None,
                )
                .expect("should apply batch");

            // Each masternode's reward is half the pools
            let reward = (storage_fees + processing_fees) / proposers_count as u64;
            let balances = platform
                .drive
                .fetch_identities_balances(
                    &vec![pro_tx_hashes[0], pro_tx_hashes[1], recipient],
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected the balances");
            // The first masternode kept its whole reward, the second's recipient got all of it
            // and the last proposer the rest of the pools (nothing)
            assert_eq!(balances.get(&pro_tx_hashes[0]), Some(&reward));
            assert_eq!(balances.get(&recipient), Some(&reward));
            assert_eq!(balances.get(&pro_tx_hashes[1]), Some(&0));
        }
    }
}
