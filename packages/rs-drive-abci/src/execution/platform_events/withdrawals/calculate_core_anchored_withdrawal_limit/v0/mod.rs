use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::balances::credits::CREDITS_PER_DUFF;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use dpp::withdrawal::core_credit_pool_unlock_limit::core_credit_pool_unlock_limit;
use drive::grovedb::TransactionArg;

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
        let system_limits = &platform_version.system_limits;
        let window_min_blocks = system_limits.core_credit_pool_window_min_blocks.ok_or(
            Error::Execution(ExecutionError::CorruptedCodeExecution(
                "calculate_core_anchored_withdrawal_limit v0 requires system_limits.core_credit_pool_window_min_blocks",
            )),
        )?;
        let window_max_blocks = system_limits.core_credit_pool_window_max_blocks.ok_or(
            Error::Execution(ExecutionError::CorruptedCodeExecution(
                "calculate_core_anchored_withdrawal_limit v0 requires system_limits.core_credit_pool_window_max_blocks",
            )),
        )?;

        let chain_locked_height = block_info.core_height;

        // The window starts of the band that exist on the chain. One before the chain's start
        // has no credit pool, which Core reads as a balance of 0: it never raises the highest.
        let window_start_balance = match chain_locked_height.checked_sub(window_min_blocks) {
            None => 0,
            Some(nearest_window_start) => {
                let farthest_window_start = chain_locked_height.saturating_sub(window_max_blocks);
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

        // Core's own limit only reflects unlocks already mined.
        let in_flight = self
            .drive
            .fetch_in_flight_withdrawal_amount(transaction, platform_version)?;

        Ok(limit.saturating_sub(in_flight))
    }

    /// Core's credit pool balance after the chain locked Core block at `core_height`, in
    /// credits, read from Core because the scan has not recorded it (yet).
    fn core_credit_pool_balance_from_core(&self, core_height: u32) -> Result<Credits, Error> {
        self.core_rpc
            .get_credit_pool_block(core_height)?
            .credit_pool_balance
            .checked_mul(CREDITS_PER_DUFF)
            .ok_or(Error::Execution(ExecutionError::Overflow(
                "core credit pool balance in credits",
            )))
    }
}

#[cfg(test)]
mod tests {
    use crate::rpc::core::{CoreCreditPoolBlock, MockCoreRPCLike};
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::dash_to_credits;
    use dpp::dashcore::consensus::Encodable;
    use dpp::dashcore::transaction::special_transaction::asset_unlock::unqualified_asset_unlock::{
        AssetUnlockBasePayload, AssetUnlockBaseTransactionInfo,
    };
    use dpp::dashcore::{ScriptBuf, TxOut};
    use dpp::version::PlatformVersion;
    use drive::util::batch::DriveOperation;

    const DUFFS_PER_DASH: u64 = 100_000_000;

    /// A Core whose credit pool balance at each height is `balance_at(height)` Dash.
    fn core_with_balances(balance_at: fn(u32) -> u64) -> MockCoreRPCLike {
        let mut core_rpc = MockCoreRPCLike::new();
        core_rpc
            .expect_get_credit_pool_block()
            .returning(move |core_height| {
                Ok(CoreCreditPoolBlock {
                    credit_pool_balance: balance_at(core_height) * DUFFS_PER_DASH,
                    asset_lock_txids: vec![],
                })
            });
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

    /// A deposit Core still counts in full is withdrawable on top; one about to leave Core's
    /// window (inside the band's near edge) already counts as if it had.
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

        // At 10,000 the band is 9,400..=9,448: the deposit counts in full.
        assert_eq!(limit(10_000), dash_to_credits!(10550));
        // At 10,052 the band's near edge reaches 9,500: Core still counts it for 24 more
        // blocks, but an unlock pooled now may be mined after it leaves Core's window.
        assert_eq!(limit(10_052), dash_to_credits!(6300));
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

        // A queued withdrawal paying out 1,000 Dash with a 1,000 duff fee.
        let untied = AssetUnlockBaseTransactionInfo {
            version: 1,
            lock_time: 0,
            output: vec![TxOut {
                value: 1_000 * DUFFS_PER_DASH,
                script_pubkey: ScriptBuf::new(),
            }],
            base_payload: AssetUnlockBasePayload {
                version: 1,
                index: 0,
                fee: 1_000,
            },
        };
        let mut bytes = vec![];
        untied
            .consensus_encode(&mut bytes)
            .expect("expected to encode");
        let mut drive_operations: Vec<DriveOperation> = vec![];
        platform
            .drive
            .add_enqueue_untied_withdrawal_transaction_operations(
                vec![(0, bytes)],
                dash_to_credits!(1000),
                &mut drive_operations,
                platform_version,
            )
            .expect("expected to enqueue");
        platform
            .drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to apply");

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
                9_420,
                dash_to_credits!(40000),
                &[],
                &block(10_000),
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
