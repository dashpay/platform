use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::withdrawal::core_credit_pool_unlock_limit::core_credit_pool_window_blocks;
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

        let chain_locked_height = block_info.core_height;

        // Nothing older than the band the limit reads is worth reading: its balance is never
        // read again.
        let oldest_useful_height = chain_locked_height.saturating_sub(
            core_credit_pool_window_blocks(self.config.network, platform_version)?,
        );

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

        let balances = (first_height..=last_height)
            .map(|core_height| {
                Ok((
                    core_height,
                    self.core_credit_pool_balance_from_core(core_height)?,
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?;

        self.drive
            .record_core_credit_pool_blocks(&balances, transaction, platform_version)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::error::execution::ExecutionError;
    use crate::error::Error;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    /// The Core heights read, in order, by a mock that answers every height with a balance of
    /// `height * 1000` duffs.
    fn recording_core(read: Arc<Mutex<Vec<u32>>>) -> MockCoreRPCLike {
        let mut core_rpc = MockCoreRPCLike::new();
        core_rpc
            .expect_get_credit_pool_balance()
            .returning(move |core_height| {
                read.lock().expect("lock").push(core_height);
                Ok(u64::from(core_height) * 1000)
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

        // First run: starts at the far edge of the band (1000 - 576, Core's mainnet window)
        // and reads 32 blocks.
        platform
            .scan_core_blocks_for_withdrawals(&block(1000), Some(&transaction), platform_version)
            .expect("expected to scan");
        assert_eq!(*read.lock().expect("lock"), (424..=455).collect::<Vec<_>>());

        // It goes on from the next unread block.
        read.lock().expect("lock").clear();
        platform
            .scan_core_blocks_for_withdrawals(&block(1000), Some(&transaction), platform_version)
            .expect("expected to scan");
        assert_eq!(*read.lock().expect("lock"), (456..=487).collect::<Vec<_>>());

        // A jump past the band skips what is too old to matter.
        read.lock().expect("lock").clear();
        platform
            .scan_core_blocks_for_withdrawals(&block(2000), Some(&transaction), platform_version)
            .expect("expected to scan");
        assert_eq!(
            *read.lock().expect("lock"),
            (1424..=1455).collect::<Vec<_>>()
        );

        // Balances are recorded in credits.
        assert_eq!(
            platform
                .drive
                .fetch_core_credit_pool_balances(1424..=1425, Some(&transaction), platform_version)
                .expect("expected the balances"),
            BTreeMap::from([(1424, 1_424_000_000), (1425, 1_425_000_000)])
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
    fn should_not_exist_before_protocol_version_14() {
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(13)
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let platform_version =
            PlatformVersion::get(13).expect("expected to get platform version 13");
        let read = Arc::new(Mutex::new(vec![]));
        platform.core_rpc = recording_core(read.clone());
        let transaction = platform.drive.grove.start_transaction();

        let result = platform.scan_core_blocks_for_withdrawals(
            &BlockInfo {
                core_height: 1000,
                ..Default::default()
            },
            Some(&transaction),
            platform_version,
        );
        assert!(matches!(
            result,
            Err(Error::Execution(ExecutionError::VersionNotActive { .. }))
        ));
        assert!(read.lock().expect("lock").is_empty());
    }
}
