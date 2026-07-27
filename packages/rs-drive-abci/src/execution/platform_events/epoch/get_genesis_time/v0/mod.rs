use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use drive::grovedb::Transaction;

impl<C> Platform<C> {
    /// Retrieves the genesis time for the specified block height and block time.
    ///
    /// # Arguments
    ///
    /// * `block_height` - The block height for which to retrieve the genesis time.
    /// * `block_time_ms` - The block time in milliseconds.
    /// * `transaction` - A reference to the transaction.
    ///
    /// # Returns
    ///
    /// * `Result<u64, Error>` - The genesis time as a `u64` value on success, or an `Error` on failure.
    pub(super) fn get_genesis_time_v0(
        &self,
        block_height: u64,
        block_time_ms: u64,
        transaction: &Transaction,
    ) -> Result<u64, Error> {
        if block_height == self.config.abci.genesis_height {
            // we do not set the genesis time to the cache here,
            // instead that must be done after finalizing the block
            Ok(block_time_ms)
        } else {
            //todo: lazy load genesis time
            self.drive
                .get_genesis_time(Some(transaction))
                .map_err(Error::Drive)?
                .ok_or(Error::Execution(ExecutionError::DriveIncoherence(
                    "the genesis time must be set",
                )))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::error::execution::ExecutionError;
    use crate::error::Error;
    use crate::test::helpers::setup::TestPlatformBuilder;

    /// At the configured genesis height, `get_genesis_time_v0` must simply echo
    /// back `block_time_ms` — it does not consult drive storage.
    #[test]
    fn v0_returns_block_time_at_genesis_height() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let genesis_height = platform.platform.config.abci.genesis_height;
        let transaction = platform.drive.grove.start_transaction();

        let got = platform
            .platform
            .get_genesis_time_v0(genesis_height, 1_234_567, &transaction)
            .expect("at genesis height must succeed");

        assert_eq!(got, 1_234_567);
    }

    /// Above genesis height with no stored genesis time, the fallback must return
    /// a `DriveIncoherence` execution error. We use `set_initial_state_structure`
    /// to ensure the tree exists but no genesis time has been cached.
    #[test]
    fn v0_above_genesis_without_stored_time_returns_drive_incoherence() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let genesis_height = platform.platform.config.abci.genesis_height;
        let transaction = platform.drive.grove.start_transaction();

        let err = platform
            .platform
            .get_genesis_time_v0(genesis_height + 1, 42, &transaction)
            .expect_err("must fail when genesis time is not set");

        match err {
            Error::Execution(ExecutionError::DriveIncoherence(msg)) => {
                assert!(
                    msg.contains("genesis time"),
                    "error must mention genesis time: {}",
                    msg
                );
            }
            other => panic!("expected DriveIncoherence error, got: {:?}", other),
        }
    }

    /// When the drive cache has a genesis time set, `get_genesis_time_v0` at a
    /// height ABOVE genesis must return the cached time — NOT the supplied
    /// `block_time_ms`. This pins the "non-genesis-height path reads from drive"
    /// contract.
    #[test]
    fn v0_above_genesis_with_cached_time_returns_drive_value() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        // Manually seed the cached genesis time so that the drive lookup hits.
        const GENESIS_TIME_MS: u64 = 123_456_789;
        platform.drive.set_genesis_time(GENESIS_TIME_MS);

        let genesis_height = platform.platform.config.abci.genesis_height;
        let transaction = platform.drive.grove.start_transaction();

        // Passing a sentinel block_time_ms that we expect to be ignored.
        let got = platform
            .platform
            .get_genesis_time_v0(genesis_height + 5, 0xdead_beef, &transaction)
            .expect("must succeed when drive has genesis time");

        assert_eq!(
            got, GENESIS_TIME_MS,
            "above-genesis must read from drive, not echo block_time_ms"
        );
    }

    /// At genesis height, even if drive has a cached genesis time, the supplied
    /// `block_time_ms` is echoed back — we do NOT consult the drive. This pins
    /// the short-circuit at the top of the function.
    #[test]
    fn v0_at_genesis_height_short_circuits_drive_lookup() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        // Seed the drive with a value that differs from block_time_ms.
        platform.drive.set_genesis_time(999_999_999);

        let genesis_height = platform.platform.config.abci.genesis_height;
        let transaction = platform.drive.grove.start_transaction();

        let got = platform
            .platform
            .get_genesis_time_v0(genesis_height, 42, &transaction)
            .expect("genesis-height path must succeed");

        assert_eq!(
            got, 42,
            "at genesis height, we must echo block_time_ms and ignore drive"
        );
    }
}
