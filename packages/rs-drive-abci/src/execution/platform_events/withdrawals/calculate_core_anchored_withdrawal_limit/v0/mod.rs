use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::identity::convert_duffs_to_credits;
use dpp::version::PlatformVersion;
use dpp::withdrawal::core_credit_pool_unlock_limit::{
    core_credit_pool_unlock_limit, core_credit_pool_window_blocks,
};
use drive::grovedb::TransactionArg;
use std::ops::RangeInclusive;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn calculate_core_anchored_withdrawal_limit_v0(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, Error> {
        // Core's asset unlock validity, which `update_broadcasted_withdrawal_statuses` also
        // expires withdrawals by.
        let unlock_validity_blocks = platform_version
            .drive_abci
            .withdrawal_constants
            .core_expiration_blocks;
        let window_blocks = core_credit_pool_window_blocks(self.config.network, platform_version)?;

        let chain_locked_height = block_info.core_height;

        // An unlock signed at request height r is mined in a block M with r < M <= r + 48 (Core
        // checks the previous block's height against r + 48) and measured from the balance
        // after M - 1 - window: a window start from r - window to r + 47 - window. Pooling at
        // chain locked height h signs at h, or at h + 1 when the chain locked height moves before
        // the later Platform block that signs it, so the window starts read run from h - window
        // to h + 48 - window. A window start before the chain's start has no credit pool, which
        // Core reads as a balance of 0: it never raises the highest.
        let window_start_balance = match chain_locked_height
            .saturating_add(unlock_validity_blocks)
            .checked_sub(window_blocks)
        {
            None => 0,
            Some(nearest_window_start) => self.highest_core_credit_pool_balance(
                chain_locked_height.saturating_sub(window_blocks)..=nearest_window_start,
                transaction,
                platform_version,
            )?,
        };

        let balance = self.highest_core_credit_pool_balance(
            chain_locked_height..=chain_locked_height,
            transaction,
            platform_version,
        )?;

        let limit = core_credit_pool_unlock_limit(balance, window_start_balance, platform_version)?;

        // Core's own limit only reflects unlocks already mined: subtract every queued and
        // broadcast one. `update_broadcasted_withdrawal_statuses` removes the ones Core mined by
        // the chain locked height from the broadcast tree in the first block at that height,
        // up to its batch of withdrawal documents, and one signed since cannot be mined by it,
        // so only a broadcast backlog beyond that batch is subtracted again after Core mined
        // it: the limit is then lower than Core's, never higher, until the statuses catch up.
        let in_flight = self
            .drive
            .fetch_in_flight_withdrawal_amount(transaction, platform_version)?;

        Ok(limit.saturating_sub(in_flight))
    }

    /// The highest of Core's credit pool balances after the chain locked Core blocks at
    /// `core_heights`, in credits: recorded by the scan where it has, read from Core otherwise.
    fn highest_core_credit_pool_balance(
        &self,
        core_heights: RangeInclusive<u32>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, Error> {
        let recorded = self.drive.fetch_core_credit_pool_balances(
            core_heights.clone(),
            transaction,
            platform_version,
        )?;
        let mut highest: Credits = 0;
        for core_height in core_heights {
            let balance = match recorded.get(&core_height) {
                Some(balance) => *balance,
                None => self.core_credit_pool_balance_from_core(core_height)?,
            };
            highest = highest.max(balance);
        }
        Ok(highest)
    }

    /// Core's credit pool balance after the chain locked Core block at `core_height`, in
    /// credits, as Core answers it.
    pub(in crate::execution::platform_events::withdrawals) fn core_credit_pool_balance_from_core(
        &self,
        core_height: u32,
    ) -> Result<Credits, Error> {
        Ok(convert_duffs_to_credits(
            self.core_rpc.get_credit_pool_balance(core_height)?,
        )?)
    }
}

#[cfg(test)]
mod tests {
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::dash_to_credits;
    use dpp::dashcore::consensus::Encodable;
    use dpp::dashcore::transaction::special_transaction::asset_unlock::unqualified_asset_unlock::{
        AssetUnlockBasePayload, AssetUnlockBaseTransactionInfo,
    };
    use dpp::dashcore::{ScriptBuf, TxOut};
    use dpp::fee::Credits;
    use dpp::version::PlatformVersion;
    use drive::grovedb::Transaction;
    use drive::util::batch::DriveOperation;
    use std::sync::{Arc, Mutex};

    const DUFFS_PER_DASH: u64 = 100_000_000;

    /// A Core whose credit pool balance at each height is `balance_at(height)` Dash.
    fn core_with_balances(balance_at: fn(u32) -> u64) -> MockCoreRPCLike {
        let mut core_rpc = MockCoreRPCLike::new();
        core_rpc
            .expect_get_credit_pool_balance()
            .returning(move |core_height| Ok(balance_at(core_height) * DUFFS_PER_DASH));
        core_rpc
    }

    fn block(core_height: u32) -> BlockInfo {
        BlockInfo {
            core_height,
            ..Default::default()
        }
    }

    #[test]
    fn should_allow_the_percent_of_an_unchanged_pool() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        platform.core_rpc = core_with_balances(|_| 37_000);
        let transaction = platform.drive.grove.start_transaction();

        assert_eq!(
            platform
                .calculate_core_anchored_withdrawal_limit(
                    &block(10_000),
                    Some(&transaction),
                    PlatformVersion::latest()
                )
                .expect("expected the limit"),
            dash_to_credits!(5550)
        );
    }

    /// The edge case the whole limit exists for: a large deposit mined a day ago is inside
    /// the window start balance, so it adds only the percent of itself, whenever Platform
    /// learns of it.
    #[test]
    fn should_add_only_the_percent_of_a_deposit_mined_before_the_band() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        // 37,000 Dash, plus a 5,000 Dash asset lock mined at Core height 9,000.
        platform.core_rpc = core_with_balances(
            |core_height| {
                if core_height >= 9_000 {
                    42_000
                } else {
                    37_000
                }
            },
        );
        let transaction = platform.drive.grove.start_transaction();

        // 15% of 42,000: the deposit adds 750, not 5,000.
        assert_eq!(
            platform
                .calculate_core_anchored_withdrawal_limit(
                    &block(10_000),
                    Some(&transaction),
                    PlatformVersion::latest()
                )
                .expect("expected the limit"),
            dash_to_credits!(6300)
        );
    }

    /// A deposit Core still counts in full is withdrawable on top; one that leaves Core's
    /// window before an unlock pooled now may be mined already counts as if it had.
    #[test]
    fn should_count_a_deposit_in_full_only_while_it_is_younger_than_the_band() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        // 37,000 Dash, plus a 5,000 Dash asset lock mined at Core height 9,500.
        platform.core_rpc = core_with_balances(
            |core_height| {
                if core_height >= 9_500 {
                    42_000
                } else {
                    37_000
                }
            },
        );
        let transaction = platform.drive.grove.start_transaction();
        let limit = |core_height: u32| {
            platform
                .calculate_core_anchored_withdrawal_limit(
                    &block(core_height),
                    Some(&transaction),
                    PlatformVersion::latest(),
                )
                .expect("expected the limit")
        };

        // At 10,000 the window starts are 9,424..=9,472: the deposit counts in full.
        assert_eq!(limit(10_000), dash_to_credits!(10550));
        assert_eq!(limit(10_027), dash_to_credits!(10550));
        // At 10,028 the nearest window start reaches 9,500: an unlock pooled now and signed one
        // Core block later may be mined at 10,077, measured from the balance after 9,500, which
        // already holds the deposit.
        assert_eq!(limit(10_028), dash_to_credits!(6300));
    }

    /// An untied withdrawal transaction paying out 1,000 Dash with a 1,000 duff fee.
    fn untied_transaction(index: u64) -> Vec<u8> {
        let untied = AssetUnlockBaseTransactionInfo {
            version: 1,
            lock_time: 0,
            output: vec![TxOut {
                value: 1_000 * DUFFS_PER_DASH,
                script_pubkey: ScriptBuf::new(),
            }],
            base_payload: AssetUnlockBasePayload {
                version: 1,
                index,
                fee: 1_000,
            },
        };
        let mut bytes = vec![];
        untied
            .consensus_encode(&mut bytes)
            .expect("expected to encode");
        bytes
    }

    /// Pools the given 1,000 Dash withdrawals, then moves the first `broadcast` of them to the
    /// broadcast tree as signing does.
    fn pool_withdrawals(
        platform: &TempPlatform<MockCoreRPCLike>,
        indices: &[u64],
        broadcast: u16,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        platform
            .drive
            .add_enqueue_untied_withdrawal_transaction_operations(
                indices
                    .iter()
                    .map(|index| (*index, untied_transaction(*index)))
                    .collect(),
                dash_to_credits!(1000) * indices.len() as u64,
                &mut drive_operations,
                platform_version,
            )
            .expect("expected to enqueue");
        if broadcast > 0 {
            platform
                .drive
                .apply_drive_operations(
                    drive_operations,
                    true,
                    &BlockInfo::default(),
                    Some(transaction),
                    platform_version,
                    None,
                )
                .expect("expected to apply");
            drive_operations = vec![];
            platform
                .drive
                .dequeue_untied_withdrawal_transactions(
                    broadcast,
                    Some(transaction),
                    &mut drive_operations,
                    platform_version,
                )
                .expect("expected to dequeue");
        }
        platform
            .drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(transaction),
                platform_version,
                None,
            )
            .expect("expected to apply");
    }

    #[test]
    fn should_subtract_what_is_pooled_and_not_mined_yet() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        platform.core_rpc = core_with_balances(|_| 37_000);
        let platform_version = PlatformVersion::latest();
        let transaction = platform.drive.grove.start_transaction();

        pool_withdrawals(&platform, &[0], 0, &transaction, platform_version);

        assert_eq!(
            platform
                .calculate_core_anchored_withdrawal_limit(
                    &block(10_000),
                    Some(&transaction),
                    platform_version
                )
                .expect("expected the limit"),
            dash_to_credits!(4550) - 1_000_000
        );
    }

    /// Queued and broadcast unlocks are both subtracted, from state alone: Core is not asked
    /// whether it mined a broadcast one (the mock has no answer for that and would panic).
    #[test]
    fn should_subtract_queued_and_broadcast_unlocks_without_asking_core() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        platform.core_rpc = core_with_balances(|_| 37_000);
        let platform_version = PlatformVersion::latest();
        let transaction = platform.drive.grove.start_transaction();

        // Indices 0 and 1 broadcast, index 2 still queued.
        pool_withdrawals(&platform, &[0, 1, 2], 2, &transaction, platform_version);

        assert_eq!(
            platform
                .calculate_core_anchored_withdrawal_limit(
                    &block(10_000),
                    Some(&transaction),
                    platform_version
                )
                .expect("expected the limit"),
            dash_to_credits!(2550) - 3_000_000
        );
    }

    /// The scan, the limit and the cleanup over a chain locked height that advances one Core
    /// block per Platform block, as in run_block_proposal: once the scan has caught up, the
    /// only balance asked of Core is the new chain locked height's, the limit follows a deposit
    /// out of the band, and the cleanup keeps exactly the heights the band can still read.
    #[test]
    fn should_read_only_the_new_core_block_once_the_scan_has_caught_up() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let platform_version = PlatformVersion::latest();
        // 37,000 Dash, plus a 5,000 Dash asset lock mined at Core height 10,000.
        let reads = Arc::new(Mutex::new(vec![]));
        let read = reads.clone();
        let mut core_rpc = MockCoreRPCLike::new();
        core_rpc
            .expect_get_credit_pool_balance()
            .returning(move |core_height| {
                read.lock().expect("lock").push(core_height);
                let dash = if core_height >= 10_000 {
                    42_000
                } else {
                    37_000
                };
                Ok(dash * DUFFS_PER_DASH)
            });
        platform.core_rpc = core_rpc;
        let transaction = platform.drive.grove.start_transaction();

        let run_block = |core_height: u32| -> Credits {
            let block_info = BlockInfo {
                time_ms: 1_000_000,
                core_height,
                ..Default::default()
            };
            platform
                .scan_core_blocks_for_withdrawals(&block_info, Some(&transaction), platform_version)
                .expect("expected to scan");
            let limit = platform
                .calculate_core_anchored_withdrawal_limit(
                    &block_info,
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected the limit");
            platform
                .clean_up_expired_locks_of_withdrawal_amounts(
                    &block_info,
                    &transaction,
                    platform_version,
                )
                .expect("expected the cleanup");
            limit
        };

        // The scan reads 32 Core blocks per Platform block: 577 from 9,424 to 10,000 take 19.
        for _ in 0..19 {
            run_block(10_000);
        }

        for core_height in 10_001..=10_600 {
            reads.lock().expect("lock").clear();
            let limit = run_block(core_height);
            assert_eq!(*reads.lock().expect("lock"), vec![core_height]);

            // The deposit counts in full until the nearest window start reaches it.
            let expected = if core_height < 10_528 {
                dash_to_credits!(10550)
            } else {
                dash_to_credits!(6300)
            };
            assert_eq!(limit, expected, "at Core height {core_height}");
        }

        let recorded: Vec<u32> = platform
            .drive
            .fetch_core_credit_pool_balances(0..=u32::MAX, Some(&transaction), platform_version)
            .expect("expected the balances")
            .into_keys()
            .collect();
        assert_eq!(recorded, (10_600 - 576..=10_600).collect::<Vec<_>>());
    }

    /// Recorded balances are read from state; Core is asked only for what is missing.
    #[test]
    fn should_prefer_recorded_balances_to_asking_core() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let platform_version = PlatformVersion::latest();
        // Core says 37,000 everywhere.
        platform.core_rpc = core_with_balances(|_| 37_000);
        let transaction = platform.drive.grove.start_transaction();

        // The scan recorded 40,000 Dash at one window start of the band.
        platform
            .drive
            .record_core_credit_pool_blocks(
                &[(9_430, dash_to_credits!(40000))],
                Some(&transaction),
                platform_version,
            )
            .expect("expected to record the block");

        // The highest window start is the recorded 40,000: 15% of it, minus the 3,000 drop.
        assert_eq!(
            platform
                .calculate_core_anchored_withdrawal_limit(
                    &block(10_000),
                    Some(&transaction),
                    platform_version
                )
                .expect("expected the limit"),
            dash_to_credits!(3000)
        );
    }

    #[test]
    fn should_treat_window_starts_before_the_chain_as_an_empty_pool() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        platform.core_rpc = core_with_balances(|_| 1_000);
        let transaction = platform.drive.grove.start_transaction();

        // Height 100 has no window start in the chain: the whole pool entered inside the
        // window and is withdrawable.
        assert_eq!(
            platform
                .calculate_core_anchored_withdrawal_limit(
                    &block(100),
                    Some(&transaction),
                    PlatformVersion::latest()
                )
                .expect("expected the limit"),
            dash_to_credits!(1000)
        );
    }
}
