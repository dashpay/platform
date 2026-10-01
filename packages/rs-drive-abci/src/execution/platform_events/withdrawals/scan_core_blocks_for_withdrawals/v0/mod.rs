use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::balances::credits::CREDITS_PER_DUFF;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::hashes::Hash;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn scan_core_blocks_for_withdrawals_v0(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let limit = platform_version
            .drive_abci
            .withdrawal_constants
            .core_blocks_scanned_per_block_limit;
        if limit == 0 {
            return Ok(());
        }

        let window_max_blocks = platform_version
            .system_limits
            .core_credit_pool_window_max_blocks
            .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "scan_core_blocks_for_withdrawals v0 requires system_limits.core_credit_pool_window_max_blocks",
            )))?;

        let chain_locked_height = block_info.core_height;

        // Nothing older than the band the limit reads is worth reading: its balance is never
        // read again, and an asset lock it mined is out of the window anyway (the pending
        // entry is dropped by the cleanup).
        let oldest_useful_height = chain_locked_height.saturating_sub(window_max_blocks);

        let first_height = match self
            .drive
            .fetch_last_recorded_core_credit_pool_height(transaction, platform_version)?
        {
            Some(last_recorded_height) => match last_recorded_height.checked_add(1) {
                Some(next_height) => next_height.max(oldest_useful_height),
                None => return Ok(()),
            },
            None => oldest_useful_height,
        };

        if first_height > chain_locked_height {
            return Ok(());
        }

        let last_height =
            chain_locked_height.min(first_height.saturating_add(u32::from(limit) - 1));

        for core_height in first_height..=last_height {
            let core_block = self.core_rpc.get_credit_pool_block(core_height)?;

            let credit_pool_balance = core_block
                .credit_pool_balance
                .checked_mul(CREDITS_PER_DUFF)
                .ok_or(Error::Execution(ExecutionError::Overflow(
                    "core credit pool balance in credits",
                )))?;

            let asset_lock_txids: Vec<[u8; 32]> = core_block
                .asset_lock_txids
                .iter()
                .map(|txid| txid.to_byte_array())
                .collect();

            self.drive.record_core_credit_pool_block(
                core_height,
                credit_pool_balance,
                &asset_lock_txids,
                block_info,
                transaction,
                platform_version,
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::rpc::core::{CoreCreditPoolBlock, MockCoreRPCLike};
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::Txid;
    use dpp::version::PlatformVersion;
    use drive::drive::identity::withdrawals::paths::{
        get_withdrawal_root_path, WITHDRAWAL_CORE_DATED_CREDIT_INFLOWS_SUM_TREE_KEY,
    };
    use drive::util::grove_operations::DirectQueryType;
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    /// The Core heights read, in order, by a mock that answers every height with a balance of
    /// `height * 1000` duffs and, at height 1003, one asset lock.
    fn recording_core(read: Arc<Mutex<Vec<u32>>>) -> MockCoreRPCLike {
        let mut core_rpc = MockCoreRPCLike::new();
        core_rpc
            .expect_get_credit_pool_block()
            .returning(move |core_height| {
                read.lock().expect("lock").push(core_height);
                Ok(CoreCreditPoolBlock {
                    credit_pool_balance: u64::from(core_height) * 1000,
                    asset_lock_txids: if core_height == 1003 {
                        vec![Txid::from_byte_array([3; 32])]
                    } else {
                        vec![]
                    },
                })
            });
        core_rpc
    }

    #[test]
    fn should_read_each_core_block_once_oldest_first_and_bounded_per_block() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let platform_version = PlatformVersion::latest();
        let read = Arc::new(Mutex::new(vec![]));
        platform.core_rpc = recording_core(read.clone());
        let transaction = platform.drive.grove.start_transaction();
        let block = |core_height: u32| BlockInfo {
            core_height,
            ..Default::default()
        };

        // First run: starts at the far edge of the band (1000 - 600) and reads 32 blocks.
        platform
            .scan_core_blocks_for_withdrawals(&block(1000), Some(&transaction), platform_version)
            .expect("expected to scan");
        assert_eq!(*read.lock().expect("lock"), (400..=431).collect::<Vec<_>>());

        // It goes on from the next unread block.
        read.lock().expect("lock").clear();
        platform
            .scan_core_blocks_for_withdrawals(&block(1000), Some(&transaction), platform_version)
            .expect("expected to scan");
        assert_eq!(*read.lock().expect("lock"), (432..=463).collect::<Vec<_>>());

        // A jump past the band skips what is too old to matter.
        read.lock().expect("lock").clear();
        platform
            .scan_core_blocks_for_withdrawals(&block(2000), Some(&transaction), platform_version)
            .expect("expected to scan");
        assert_eq!(
            *read.lock().expect("lock"),
            (1400..=1431).collect::<Vec<_>>()
        );

        // Balances are recorded in credits.
        assert_eq!(
            platform
                .drive
                .fetch_core_credit_pool_balances(1400..=1401, Some(&transaction), platform_version)
                .expect("expected the balances"),
            BTreeMap::from([(1400, 1_400_000_000), (1401, 1_401_000_000)])
        );
    }

    #[test]
    fn should_read_nothing_once_caught_up_with_the_chain_locked_height() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let platform_version = PlatformVersion::latest();
        let read = Arc::new(Mutex::new(vec![]));
        platform.core_rpc = recording_core(read.clone());
        let transaction = platform.drive.grove.start_transaction();
        let block = |core_height: u32| BlockInfo {
            core_height,
            ..Default::default()
        };

        // A young chain: everything from height 0 is in the band.
        platform
            .scan_core_blocks_for_withdrawals(&block(5), Some(&transaction), platform_version)
            .expect("expected to scan");
        assert_eq!(*read.lock().expect("lock"), (0..=5).collect::<Vec<_>>());

        read.lock().expect("lock").clear();
        platform
            .scan_core_blocks_for_withdrawals(&block(5), Some(&transaction), platform_version)
            .expect("expected to scan");
        assert!(read.lock().expect("lock").is_empty());

        platform
            .scan_core_blocks_for_withdrawals(&block(7), Some(&transaction), platform_version)
            .expect("expected to scan");
        assert_eq!(*read.lock().expect("lock"), vec![6, 7]);
    }

    #[test]
    fn should_date_a_pending_asset_lock_by_the_core_block_that_holds_it() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let platform_version = PlatformVersion::latest();
        let read = Arc::new(Mutex::new(vec![]));
        platform.core_rpc = recording_core(read.clone());
        let transaction = platform.drive.grove.start_transaction();
        let block = |core_height: u32| BlockInfo {
            time_ms: 5_000,
            core_height,
            ..Default::default()
        };

        // Caught up to Core height 1001, then the asset lock is consumed before Core mines it.
        platform
            .drive
            .record_core_credit_pool_block(
                1001,
                0,
                &[],
                &block(1001),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to record the block");
        platform
            .drive
            .record_asset_lock_credit_inflow(
                [3; 32],
                700,
                None,
                &block(1001),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to record the pending inflow");

        let core_dated_inflows = |transaction| {
            platform
                .drive
                .grove_get_sum_tree_total_value(
                    (&get_withdrawal_root_path()).into(),
                    &WITHDRAWAL_CORE_DATED_CREDIT_INFLOWS_SUM_TREE_KEY,
                    DirectQueryType::StatefulDirectQuery,
                    Some(transaction),
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected the sum")
        };
        assert_eq!(core_dated_inflows(&transaction), 0);

        // Core height 1003 mined it.
        platform
            .scan_core_blocks_for_withdrawals(&block(1003), Some(&transaction), platform_version)
            .expect("expected to scan");
        assert_eq!(*read.lock().expect("lock"), vec![1002, 1003]);
        assert_eq!(core_dated_inflows(&transaction), 700);
    }

    #[test]
    fn should_do_nothing_before_the_feature_exists() {
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(13)
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let platform_version =
            PlatformVersion::get(13).expect("expected to get platform version 13");
        let read = Arc::new(Mutex::new(vec![]));
        platform.core_rpc = recording_core(read.clone());
        let transaction = platform.drive.grove.start_transaction();

        platform
            .scan_core_blocks_for_withdrawals(
                &BlockInfo {
                    core_height: 1000,
                    ..Default::default()
                },
                Some(&transaction),
                platform_version,
            )
            .expect("expected the event to be a no-op before v14");
        assert!(read.lock().expect("lock").is_empty());
    }
}
