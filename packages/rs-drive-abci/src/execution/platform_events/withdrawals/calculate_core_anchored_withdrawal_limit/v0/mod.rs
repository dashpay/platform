use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore_rpc::dashcore_rpc_json::AssetUnlockStatus;
use dpp::fee::Credits;
use dpp::identity::convert_duffs_to_credits;
use dpp::version::PlatformVersion;
use dpp::withdrawal::core_credit_pool_unlock_limit::{
    core_credit_pool_unlock_limit, core_credit_pool_window_blocks,
};
use dpp::withdrawal::WithdrawalTransactionIndex;
use drive::grovedb::TransactionArg;

/// The most asset unlock indexes Core's `getassetunlockstatuses` answers in one call.
const MAX_ASSET_UNLOCK_STATUSES_PER_REQUEST: usize = 100;

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
        let mining_delay_blocks = platform_version
            .system_limits
            .core_credit_pool_unlock_mining_delay_blocks
            .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "calculate_core_anchored_withdrawal_limit v0 requires system_limits.core_credit_pool_unlock_mining_delay_blocks",
            )))?;
        let window_blocks = core_credit_pool_window_blocks(self.config.network, platform_version)?;

        let chain_locked_height = block_info.core_height;

        // Core measures an unlock mined in block M from the balance after M - 1 - window; M is
        // past the chain locked height and at most `mining_delay_blocks` past it (the unlock
        // is signed at it or later). A window start before the chain's start has no credit
        // pool, which Core reads as a balance of 0: it never raises the highest.
        let window_start_balance = match chain_locked_height
            .saturating_add(mining_delay_blocks)
            .checked_sub(window_blocks)
        {
            None => 0,
            Some(nearest_window_start) => {
                let farthest_window_start = chain_locked_height.saturating_sub(window_blocks);
                let recorded = self.drive.fetch_core_credit_pool_balances(
                    farthest_window_start..=nearest_window_start,
                    transaction,
                    platform_version,
                )?;
                let mut highest: Credits = 0;
                for core_height in farthest_window_start..=nearest_window_start {
                    let balance = match recorded.get(&core_height) {
                        Some(balance) => *balance,
                        None => self.core_credit_pool_balance_from_core(core_height)?,
                    };
                    highest = highest.max(balance);
                }
                highest
            }
        };

        let balance = match self
            .drive
            .fetch_core_credit_pool_balances(
                chain_locked_height..=chain_locked_height,
                transaction,
                platform_version,
            )?
            .get(&chain_locked_height)
        {
            Some(balance) => *balance,
            None => self.core_credit_pool_balance_from_core(chain_locked_height)?,
        };

        let limit = core_credit_pool_unlock_limit(balance, window_start_balance, platform_version)?;

        // Core's own limit only reflects unlocks already mined: subtract the queued ones and
        // the broadcast ones Core has not mined by the chain locked height. The broadcast tree
        // keeps mined ones until their documents are updated, a bounded number per Core block.
        let in_flight = self
            .drive
            .fetch_in_flight_withdrawal_amount(transaction, platform_version)?;
        let broadcast_indices: Vec<WithdrawalTransactionIndex> =
            in_flight.broadcast.keys().copied().collect();
        let mut not_mined = in_flight.queued;
        for indices in broadcast_indices.chunks(MAX_ASSET_UNLOCK_STATUSES_PER_REQUEST) {
            let statuses = self.fetch_transactions_block_inclusion_status(
                chain_locked_height,
                indices,
                platform_version,
            )?;
            for index in indices {
                if statuses.get(index) != Some(&AssetUnlockStatus::Chainlocked) {
                    let amount = in_flight.broadcast.get(index).copied().unwrap_or_default();
                    not_mined = not_mined.saturating_add(amount);
                }
            }
        }

        Ok(limit.saturating_sub(not_mined))
    }

    /// Core's credit pool balance after the chain locked Core block at `core_height`, in
    /// credits, read from Core because the scan has not recorded it (yet).
    fn core_credit_pool_balance_from_core(&self, core_height: u32) -> Result<Credits, Error> {
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
    use dpp::dashcore_rpc::dashcore_rpc_json::{AssetUnlockStatus, AssetUnlockStatusResult};
    use dpp::version::PlatformVersion;
    use drive::grovedb::Transaction;
    use drive::util::batch::DriveOperation;

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
        // At 10,028 the nearest window start reaches 9,500: an unlock pooled now may be mined
        // 48 blocks later, when Core's window no longer holds the deposit.
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

    /// Core's balance at the chain locked height already reflects a broadcast unlock it mined
    /// by then, even while the broadcast tree still holds it.
    #[test]
    fn should_not_subtract_a_broadcast_unlock_core_already_mined() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let mut core_rpc = core_with_balances(|_| 37_000);
        core_rpc
            .expect_get_asset_unlock_statuses()
            .withf(|_, core_height| *core_height == 10_000)
            .returning(|indices, _| {
                Ok(indices
                    .iter()
                    .map(|index| AssetUnlockStatusResult {
                        index: *index,
                        status: if *index == 0 {
                            AssetUnlockStatus::Chainlocked
                        } else {
                            AssetUnlockStatus::Mempooled
                        },
                    })
                    .collect())
            });
        platform.core_rpc = core_rpc;
        let platform_version = PlatformVersion::latest();
        let transaction = platform.drive.grove.start_transaction();

        // Index 0 is mined, index 1 broadcast and not mined, index 2 still queued.
        pool_withdrawals(&platform, &[0, 1, 2], 2, &transaction, platform_version);

        assert_eq!(
            platform
                .calculate_core_anchored_withdrawal_limit(
                    &block(10_000),
                    Some(&transaction),
                    platform_version
                )
                .expect("expected the limit"),
            dash_to_credits!(3550) - 2_000_000
        );
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
            .record_core_credit_pool_block(
                9_430,
                dash_to_credits!(40000),
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
