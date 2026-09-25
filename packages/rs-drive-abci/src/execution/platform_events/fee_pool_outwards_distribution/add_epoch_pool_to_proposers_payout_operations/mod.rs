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
    /// * `Result<(StorageAndProcessingPoolCredits, BTreeMap<Identifier, u64>), Error>` - The unpaid
    ///   epoch's pool credits and the block count of every proposer paid, otherwise an `Error`.
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
    use crate::test::helpers::fee_pools::{
        create_test_mn_share_document, test_mn_share_document_id,
    };
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::balances::total_credits_balance::TotalCreditsBalance;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::data_contracts::SystemDataContract;
    use dpp::fee::Credits;
    use dpp::identifier::Identifier;
    use dpp::system_data_contracts::load_system_data_contract;
    use dpp::version::PlatformVersion;
    use dpp::version::ProtocolVersion;
    use drive::drive::credit_pools::epochs::operations_factory::EpochOperations;
    use drive::drive::identity::IdentityRootStructure;
    use drive::drive::RootTree;
    use drive::grovedb::Element;
    use drive::grovedb_path::SubtreePath;
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
    /// An id no identity has.
    const NOBODY: [u8; 32] = [5; 32];

    /// The first two proposers name the same recipient (30% and 20%) and the third names the
    /// first proposer (10%).
    const SHARED_RECIPIENT_SHARES: [([u8; 32], [u8; 32], u16); 3] = [
        (FIRST_PROPOSER, RECIPIENT, 3000),
        (SECOND_PROPOSER, RECIPIENT, 2000),
        (THIRD_PROPOSER, FIRST_PROPOSER, 1000),
    ];

    /// The state after one payout of an epoch in which each of the three proposers proposed
    /// one block.
    struct Payout {
        balances: BTreeMap<[u8; 32], Credits>,
        /// What each masternode earned, before any reward share.
        masternode_payout: Credits,
        /// The rounding remainder, which the last proposer also takes.
        remainder: Credits,
        recipient_debt_after: Credits,
        credits_after: TotalCreditsBalance,
    }

    impl Payout {
        fn share(&self, percentage: Credits) -> Credits {
            self.masternode_payout * percentage / 10000
        }

        /// What `SHARED_RECIPIENT_SHARES` owe each identity, less the recipient's debt.
        fn owed_for_shared_recipient_shares(
            &self,
            recipient_debt: Credits,
        ) -> BTreeMap<[u8; 32], Credits> {
            let m = self.masternode_payout;
            BTreeMap::from([
                (FIRST_PROPOSER, m - self.share(3000) + self.share(1000)),
                (SECOND_PROPOSER, m - self.share(2000)),
                (THIRD_PROPOSER, self.remainder + m - self.share(1000)),
                (
                    RECIPIENT,
                    self.share(3000) + self.share(2000) - recipient_debt,
                ),
            ])
        }
    }

    fn recipient_path() -> Vec<Vec<u8>> {
        vec![vec![RootTree::Identities as u8], RECIPIENT.to_vec()]
    }

    fn pay_out(
        protocol_version: ProtocolVersion,
        shares: &[([u8; 32], [u8; 32], u16)],
        recipient_debt: Credits,
    ) -> Payout {
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

        for (seed, id) in [FIRST_PROPOSER, SECOND_PROPOSER, THIRD_PROPOSER, RECIPIENT]
            .into_iter()
            .enumerate()
        {
            create_test_identity(
                &platform.drive,
                id,
                Some(seed as u64),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to create an identity");
        }
        increment_in_epoch_each_proposers_block_count(
            &platform.drive,
            &unpaid_epoch,
            &vec![FIRST_PROPOSER, SECOND_PROPOSER, THIRD_PROPOSER],
            Some(&transaction),
            platform_version,
        );

        if recipient_debt > 0 {
            let mut batch = GroveDbOpBatch::new();
            batch.add_insert(
                recipient_path(),
                vec![IdentityRootStructure::IdentityTreeNegativeCredit as u8],
                Element::new_item(recipient_debt.to_be_bytes().to_vec()),
            );
            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("expected to record the recipient's debt");
        }

        let contract =
            load_system_data_contract(SystemDataContract::MasternodeRewards, platform_version)
                .expect("expected the masternode reward shares contract");
        for (owner, pay_to, percentage) in shares {
            create_test_mn_share_document(
                &platform.drive,
                &contract,
                Identifier::new(*owner),
                Identifier::new(*pay_to),
                *percentage,
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
                // The block that pays epoch 0 out is in epoch 1.
                &BlockInfo::default_with_epoch(current_epoch),
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

        let balances = platform
            .drive
            .fetch_identities_balances(
                &vec![
                    FIRST_PROPOSER,
                    SECOND_PROPOSER,
                    THIRD_PROPOSER,
                    RECIPIENT,
                    NOBODY,
                ],
                Some(&transaction),
                platform_version,
            )
            .expect("expected the balances");

        let recipient_path = recipient_path();
        let recipient_debt_after = match platform
            .drive
            .grove
            .get(
                SubtreePath::from(recipient_path.as_slice()),
                &[IdentityRootStructure::IdentityTreeNegativeCredit as u8],
                Some(&transaction),
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("expected the recipient's negative credit")
        {
            Element::Item(bytes, _) => Credits::from_be_bytes(
                bytes
                    .try_into()
                    .expect("expected the negative credit to be 8 bytes"),
            ),
            element => panic!("expected the negative credit to be an item, got {element:?}"),
        };

        Payout {
            balances,
            masternode_payout,
            remainder: total_payouts - 3 * masternode_payout,
            recipient_debt_after,
            credits_after,
        }
    }

    fn assert_credits_balance(payout: &Payout) {
        assert!(
            payout.credits_after.ok().expect("expected no overflow"),
            "the credits must balance after the payout: {}",
            payout.credits_after
        );
    }

    #[test]
    fn should_credit_an_identity_everything_one_payout_owes_it() {
        let payout = pay_out(
            PlatformVersion::latest().protocol_version,
            &SHARED_RECIPIENT_SHARES,
            0,
        );

        assert_eq!(payout.balances, payout.owed_for_shared_recipient_shares(0));
        assert_credits_balance(&payout);
    }

    #[test]
    fn should_repay_a_recipients_debt_once_from_everything_one_payout_owes_it() {
        let recipient_debt = 1_000_000;
        let payout = pay_out(
            PlatformVersion::latest().protocol_version,
            &SHARED_RECIPIENT_SHARES,
            recipient_debt,
        );

        // One credit per identity: the debt comes out of the recipient's two shares once, and
        // what is left of them is its balance. The repaid debt goes to the block epoch's
        // processing fee pool, so the credits still balance.
        assert!(recipient_debt < payout.share(2000));
        assert_eq!(
            payout.balances,
            payout.owed_for_shared_recipient_shares(recipient_debt)
        );
        assert_eq!(payout.recipient_debt_after, 0);
        assert_credits_balance(&payout);
    }

    #[test]
    fn should_leave_a_share_naming_no_identity_with_its_masternode() {
        let payout = pay_out(
            PlatformVersion::latest().protocol_version,
            &[(FIRST_PROPOSER, NOBODY, 3000)],
            0,
        );

        let m = payout.masternode_payout;
        assert_eq!(
            payout.balances,
            BTreeMap::from([
                (FIRST_PROPOSER, m),
                (SECOND_PROPOSER, m),
                (THIRD_PROPOSER, m + payout.remainder),
                (RECIPIENT, 0),
            ])
        );
        assert_credits_balance(&payout);
    }

    #[test]
    fn should_pay_every_reward_share_of_one_masternode() {
        let payout = pay_out(
            PlatformVersion::latest().protocol_version,
            &[
                (FIRST_PROPOSER, RECIPIENT, 3000),
                (FIRST_PROPOSER, SECOND_PROPOSER, 2000),
            ],
            0,
        );

        let m = payout.masternode_payout;
        assert_eq!(
            payout.balances,
            BTreeMap::from([
                (FIRST_PROPOSER, m - payout.share(3000) - payout.share(2000)),
                (SECOND_PROPOSER, m + payout.share(2000)),
                (THIRD_PROPOSER, m + payout.remainder),
                (RECIPIENT, payout.share(3000)),
            ])
        );
        assert_credits_balance(&payout);
    }

    #[test]
    fn should_pay_shares_over_the_whole_payout_until_it_is_used_up() {
        let to_recipient = (FIRST_PROPOSER, RECIPIENT, 8000);
        let to_second_proposer = (FIRST_PROPOSER, SECOND_PROPOSER, 5000);
        // A masternode's shares are read in document id order, whatever order they were
        // written in.
        let recipient_share_read_first =
            test_mn_share_document_id(Identifier::new(FIRST_PROPOSER), Identifier::new(RECIPIENT))
                < test_mn_share_document_id(
                    Identifier::new(FIRST_PROPOSER),
                    Identifier::new(SECOND_PROPOSER),
                );

        for shares in [
            [to_recipient, to_second_proposer],
            [to_second_proposer, to_recipient],
        ] {
            let payout = pay_out(PlatformVersion::latest().protocol_version, &shares, 0);

            // 130% of the first masternode's payout is shared: the share read first is paid
            // in full, the other gets what is left, and the masternode keeps nothing.
            let m = payout.masternode_payout;
            let (recipient_share, second_proposer_share) = if recipient_share_read_first {
                (payout.share(8000), m - payout.share(8000))
            } else {
                (m - payout.share(5000), payout.share(5000))
            };
            assert_eq!(
                payout.balances,
                BTreeMap::from([
                    (FIRST_PROPOSER, 0),
                    (SECOND_PROPOSER, m + second_proposer_share),
                    (THIRD_PROPOSER, m + payout.remainder),
                    (RECIPIENT, recipient_share),
                ]),
                "shares written in the order {shares:?}"
            );
            assert_credits_balance(&payout);
        }
    }

    /// Reproduces the lost credits on generation 0 as it shipped. No reward share can be
    /// written at the protocol versions that select generation 0, so if it is ever hardened in
    /// place, this test goes with it.
    #[test]
    fn should_reproduce_the_lost_credits_of_generation_0_at_protocol_version_13() {
        let payout = pay_out(13, &SHARED_RECIPIENT_SHARES, 0);

        // Generation 0 writes one balance per credit, each from the balance before the batch,
        // and only the last write to an identity lands.
        let mut missing: Credits = 0;
        for (id, owed) in payout.owed_for_shared_recipient_shares(0) {
            let balance = payout.balances[&id];
            if id == RECIPIENT || id == FIRST_PROPOSER {
                assert!(balance < owed, "identity {id:?} should be missing a credit");
            } else {
                assert_eq!(balance, owed, "identity {id:?} has one credit");
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
