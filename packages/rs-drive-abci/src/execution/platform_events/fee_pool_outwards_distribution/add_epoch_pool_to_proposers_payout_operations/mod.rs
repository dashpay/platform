use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::unpaid_epoch::UnpaidEpoch;
use crate::platform_types::platform::Platform;
use dpp::block::pool_credits::StorageAndProcessingPoolCredits;
use std::collections::BTreeMap;

use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;
use drive::util::batch::DriveOperation;

mod v0;
mod v1;

impl<C> Platform<C> {
    /// Adds operations to the op batch which distribute the fees from an unpaid epoch pool
    /// to the total fees to be paid out to proposers and divides amongst masternode reward shares.
    ///
    /// This function is a version handler that directs to specific version implementations
    /// of the add_epoch_pool_to_proposers_payout_operations function.
    ///
    /// # Arguments
    ///
    /// * `unpaid_epoch` - A reference to an `UnpaidEpoch`.
    /// * `core_block_rewards` - A `Credits` value representing the core block rewards.
    /// * `transaction` - A `Transaction` reference.
    /// * `batch` - A mutable reference to a vector of `DriveOperation`.
    /// * `platform_version` - A `PlatformVersion` reference that dictates which version of
    ///   the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<u16, Error>` - Returns the number of proposers to be paid out if successful, otherwise returns an `Error`.
    pub(super) fn add_epoch_pool_to_proposers_payout_operations(
        &self,
        unpaid_epoch: &UnpaidEpoch,
        core_block_rewards: Credits,
        transaction: &Transaction,
        batch: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(StorageAndProcessingPoolCredits, BTreeMap<Identifier, u64>), Error> {
        match platform_version
            .drive_abci
            .methods
            .fee_pool_outwards_distribution
            .add_epoch_pool_to_proposers_payout_operations
        {
            0 => self.add_epoch_pool_to_proposers_payout_operations_v0(
                unpaid_epoch,
                core_block_rewards,
                transaction,
                batch,
                platform_version,
            ),
            1 => self.add_epoch_pool_to_proposers_payout_operations_v1(
                unpaid_epoch,
                core_block_rewards,
                transaction,
                batch,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "add_epoch_pool_to_proposers_payout_operations".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::platform_types::platform_state::PlatformStateV0Methods;
    use crate::test::helpers::fee_pools::create_test_mn_share_document;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::balances::total_credits_balance::TotalCreditsBalance;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::data_contracts::SystemDataContract;
    use dpp::fee::Credits;
    use dpp::identifier::Identifier;
    use dpp::system_data_contracts::load_system_data_contract;
    use dpp::version::PlatformVersion;
    use drive::drive::credit_pools::epochs::operations_factory::EpochOperations;
    use drive::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use drive::util::batch::GroveDbOpBatch;
    use drive::util::test_helpers::test_utils::identities::{
        create_test_identity, increment_in_epoch_each_proposers_block_count,
    };
    use std::collections::BTreeMap;

    const FIRST_PROPOSER: [u8; 32] = [1; 32];
    const SECOND_PROPOSER: [u8; 32] = [2; 32];
    const THIRD_PROPOSER: [u8; 32] = [3; 32];
    const RECIPIENT: [u8; 32] = [4; 32];

    /// Balances after one payout of an epoch in which each of three proposers proposed one
    /// block, the first two name the same recipient in their reward share (30% and 20%) and
    /// the third names the first proposer (10%).
    struct SharedRecipientPayout {
        balances: BTreeMap<[u8; 32], Credits>,
        owed: BTreeMap<[u8; 32], Credits>,
        credits_after: TotalCreditsBalance,
    }

    fn pay_out_to_shared_recipients(protocol_version: u32) -> SharedRecipientPayout {
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state_with_activation_info(0, 1);
        assert!(
            !platform.drive.config.batching_consistency_verification,
            "the payout must run on the batching configuration a node ships with"
        );
        let platform_state = platform.state.load();
        let platform_version = platform_state
            .current_platform_version()
            .expect("expected a platform version");
        assert_eq!(platform_version.protocol_version, protocol_version);
        let transaction = platform.drive.grove.start_transaction();

        let processing_fees: Credits = 100_000_000;
        let storage_fees: Credits = 10_000_000;
        let unpaid_epoch = Epoch::new(0).expect("expected epoch 0");
        let current_epoch = Epoch::new(1).expect("expected epoch 1");
        let fee_multiplier = platform_version
            .fee_version
            .uses_version_fee_multiplier_permille
            .expect("expected a fee multiplier");

        let mut batch = GroveDbOpBatch::new();
        unpaid_epoch.add_init_current_operations(
            fee_multiplier,
            1,
            1,
            1,
            platform_version.protocol_version,
            &mut batch,
        );
        batch.push(
            unpaid_epoch
                .update_processing_fee_pool_operation(processing_fees)
                .expect("expected a processing pool operation"),
        );
        batch.push(
            unpaid_epoch
                .update_storage_fee_pool_operation(storage_fees)
                .expect("expected a storage pool operation"),
        );
        // Three blocks in epoch 0, one per proposer.
        current_epoch.add_init_current_operations(
            fee_multiplier,
            4,
            3,
            2,
            platform_version.protocol_version,
            &mut batch,
        );
        platform
            .drive
            .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
            .expect("expected to set up the epochs");
        platform
            .drive
            .add_to_system_credits(
                processing_fees + storage_fees,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to count the pooled fees");

        let mut identities = BTreeMap::new();
        for (seed, id) in [FIRST_PROPOSER, SECOND_PROPOSER, THIRD_PROPOSER, RECIPIENT]
            .into_iter()
            .enumerate()
        {
            let identity = create_test_identity(
                &platform.drive,
                id,
                Some(seed as u64),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to create an identity");
            identities.insert(id, identity);
        }
        increment_in_epoch_each_proposers_block_count(
            &platform.drive,
            &unpaid_epoch,
            &vec![FIRST_PROPOSER, SECOND_PROPOSER, THIRD_PROPOSER],
            Some(&transaction),
            platform_version,
        );

        let contract =
            load_system_data_contract(SystemDataContract::MasternodeRewards, platform_version)
                .expect("expected the masternode reward shares contract");
        for (owner, pay_to, percentage) in [
            (FIRST_PROPOSER, RECIPIENT, 3000),
            (SECOND_PROPOSER, RECIPIENT, 2000),
            (THIRD_PROPOSER, FIRST_PROPOSER, 1000),
        ] {
            create_test_mn_share_document(
                &platform.drive,
                &contract,
                Identifier::new(owner),
                &identities[&pay_to],
                percentage,
                Some(&transaction),
                platform_version,
            );
        }

        let credits_before = platform
            .drive
            .calculate_total_credits_balance(Some(&transaction), &platform_version.drive)
            .expect("expected to sum the credits");
        assert!(
            credits_before.ok().expect("expected no overflow"),
            "the credits must balance before the payout: {credits_before}"
        );

        let mut batch = vec![];
        let payouts = platform
            .add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations(
                current_epoch.index,
                None,
                None,
                0,
                &transaction,
                &mut batch,
                platform_version,
            )
            .expect("expected to distribute the epoch")
            .expect("expected a payout");
        assert_eq!(payouts.proposers_paid_count, 3);
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
            .expect("expected to apply the payout");

        let credits_after = platform
            .drive
            .calculate_total_credits_balance(Some(&transaction), &platform_version.drive)
            .expect("expected to sum the credits");

        // Everything paid out: the pooled fees and the core block rewards the distribution
        // added to the system credits.
        let core_block_rewards =
            credits_after.total_credits_in_platform - credits_before.total_credits_in_platform;
        let total_payouts = processing_fees + storage_fees + core_block_rewards;
        let masternode_payout = total_payouts / 3;
        let share = |percentage: Credits| masternode_payout * percentage / 10000;
        let owed = BTreeMap::from([
            (
                FIRST_PROPOSER,
                masternode_payout - share(3000) + share(1000),
            ),
            (SECOND_PROPOSER, masternode_payout - share(2000)),
            // The last proposer also takes the rounding remainder.
            (
                THIRD_PROPOSER,
                (total_payouts - 3 * masternode_payout) + (masternode_payout - share(1000)),
            ),
            (RECIPIENT, share(3000) + share(2000)),
        ]);

        let balances = platform
            .drive
            .fetch_identities_balances(
                &owed.keys().copied().collect(),
                Some(&transaction),
                platform_version,
            )
            .expect("expected the balances");

        SharedRecipientPayout {
            balances,
            owed,
            credits_after,
        }
    }

    #[test]
    fn should_credit_an_identity_everything_one_payout_owes_it() {
        let payout = pay_out_to_shared_recipients(PlatformVersion::latest().protocol_version);

        assert_eq!(payout.balances, payout.owed);
        assert!(
            payout.credits_after.ok().expect("expected no overflow"),
            "the credits must balance after the payout: {}",
            payout.credits_after
        );
    }

    #[test]
    fn should_keep_one_credit_per_identity_at_protocol_version_13() {
        let payout = pay_out_to_shared_recipients(13);

        // Generation 0 writes one balance per credit, each from the balance before the batch,
        // and only the last write to an identity lands.
        let mut missing: Credits = 0;
        for (id, owed) in &payout.owed {
            let balance = payout.balances[id];
            if *id == RECIPIENT || *id == FIRST_PROPOSER {
                assert!(
                    balance < *owed,
                    "identity {id:?} should be missing a credit"
                );
            } else {
                assert_eq!(balance, *owed, "identity {id:?} has one credit");
            }
            missing += owed - balance;
        }
        assert!(!payout.credits_after.ok().expect("expected no overflow"));
        assert_eq!(
            payout.credits_after.total_credits_in_platform
                - payout
                    .credits_after
                    .total_in_trees()
                    .expect("expected no overflow"),
            missing
        );
    }
}
