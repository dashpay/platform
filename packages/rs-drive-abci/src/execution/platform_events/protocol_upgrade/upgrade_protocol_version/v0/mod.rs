use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::epoch_info::v0::{EpochInfoV0Getters, EpochInfoV0Methods};
use crate::platform_types::epoch_info::EpochInfo;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use chrono::{TimeZone, Utc};
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::Network::Testnet;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C> {
    /// Sets current protocol version and next epoch protocol version to block platform state
    ///
    /// It takes five parameters:
    /// * `epoch_info`: Information about the current epoch.
    /// * `last_committed_platform_state`: The last committed state of the platform.
    /// * `block_platform_state`: The current state of the platform.
    /// * `transaction`: The current transaction.
    /// * `platform_version`: The current version of the platform.
    ///
    /// # Errors
    ///
    /// This function will return an error if the previous block protocol version does not match the current block protocol version not on epoch change
    pub(super) fn upgrade_protocol_version_on_epoch_change_v0(
        &self,
        block_info: &BlockInfo,
        epoch_info: &EpochInfo,
        last_committed_platform_state: &PlatformState,
        block_platform_state: &mut PlatformState,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let previous_block_protocol_version = last_committed_platform_state
            .current_platform_version()?
            .protocol_version;
        let current_block_protocol_version = platform_version.protocol_version;

        // Protocol version can be changed only on epoch change
        if epoch_info.is_epoch_change_but_not_genesis() {
            if current_block_protocol_version == previous_block_protocol_version {
                tracing::info!(
                    epoch_index = epoch_info.current_epoch_index(),
                    "protocol version remains the same {}",
                    current_block_protocol_version,
                );
            } else {
                tracing::info!(
                    epoch_index = epoch_info.current_epoch_index(),
                    "protocol version changed from {} to {}",
                    previous_block_protocol_version,
                    current_block_protocol_version,
                );
            };

            // Determine a new protocol version for the next epoch if enough proposers voted
            // otherwise keep the current one

            let hpmn_active_list_len =
                if self.config.network == Testnet && epoch_info.current_epoch_index() <= 1430 {
                    // We had a bug on testnet that would use the entire hpmn list len, including banned nodes
                    last_committed_platform_state.hpmn_list_len() as u32
                } else {
                    last_committed_platform_state.hpmn_active_list_len() as u32
                };

            let next_epoch_protocol_version =
                self.check_for_desired_protocol_upgrade(hpmn_active_list_len, platform_version)?;

            if let Some(protocol_version) = next_epoch_protocol_version {
                tracing::trace!(
                    current_epoch_index = epoch_info.current_epoch_index(),
                    "Next protocol version set to {}",
                    protocol_version
                );

                block_platform_state.set_next_epoch_protocol_version(protocol_version);
            } else {
                tracing::trace!(
                    current_epoch_index = epoch_info.current_epoch_index(),
                    "Non of the votes reached threshold. Next protocol version remains the same {}",
                    block_platform_state.next_epoch_protocol_version()
                );
            }

            // Since we are starting a new epoch we need to drop previously
            // proposed versions

            // Remove previously proposed versions from Drive state
            self.drive
                .clear_version_information(Some(transaction), &platform_version.drive)
                .map_err(Error::Drive)?;

            let previous_fee_versions_map = block_platform_state.previous_fee_versions_mut();

            let platform_version = PlatformVersion::get(current_block_protocol_version)?;
            // If cached_fee_version is non-empty
            if let Some((_, last_fee_version)) = previous_fee_versions_map.iter().last() {
                // Insert the new (epoch_index, fee_version) only if the new fee_version is different from the last_fee_version.
                if last_fee_version.fee_version_number
                    != platform_version.fee_version.fee_version_number
                {
                    previous_fee_versions_map.insert(
                        epoch_info.current_epoch_index(),
                        &platform_version.fee_version,
                    );
                }
            // In case of empty cached_fee_version, insert the new (epoch_index, fee_version)
            } else {
                previous_fee_versions_map.insert(
                    epoch_info.current_epoch_index(),
                    &platform_version.fee_version,
                );
            }

            // We clean voting counter cache only on finalize block because:
            // 1. The voting counter global cache uses for querying of voting information in Drive queries
            // 2. There might be multiple rounds so on the next round we will lose all previous epoch vote_choices
            //
            // Instead of clearing cache, the further block processing logic is using `get_if_enabled`
            // to get a version counter from the global cache. We disable this getter here to prevent
            // reading previous voting information for the new epoch.
            // The getter must be enabled back on finalize block in [update_drive_cache] and at the very beginning
            // of the block processing in [clear_drive_block_cache].

            let mut protocol_versions_counter = self.drive.cache.protocol_versions_counter.write();
            protocol_versions_counter.block_global_cache();
            drop(protocol_versions_counter);
        } else if current_block_protocol_version != previous_block_protocol_version {
            return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "unexpected protocol upgrade: it should happen only on epoch change",
            )));
        }

        // Warn user to update software if the next protocol version is not supported
        let latest_supported_protocol_version = PlatformVersion::latest().protocol_version;
        let next_epoch_protocol_version = block_platform_state.next_epoch_protocol_version();
        if block_platform_state.next_epoch_protocol_version() > latest_supported_protocol_version {
            let genesis_time_ms = self.get_genesis_time(
                block_info.height,
                block_info.time_ms,
                transaction,
                platform_version,
            )?;

            let next_epoch_activation_datetime_ms = genesis_time_ms
                + (epoch_info.current_epoch_index() as u64
                    * self.config.execution.epoch_time_length_s
                    * 1000);

            let next_epoch_activation_datetime = Utc
                .timestamp_millis_opt(next_epoch_activation_datetime_ms as i64)
                .single()
                .expect("next_epoch_activation_date must always be in the range");

            tracing::warn!(
                next_epoch_protocol_version,
                latest_supported_protocol_version,
                "The node doesn't support new protocol version {} that will be activated starting from {}. Please update your software, otherwise the node won't be able to participate in the network. https://docs.dash.org/platform-protocol-upgrade",
                next_epoch_protocol_version,
                next_epoch_activation_datetime.to_rfc2822(),
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_types::epoch_info::v0::EpochInfoV0;
    use crate::platform_types::epoch_info::EpochInfo;
    use crate::platform_types::platform_state::PlatformStateV0Methods;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::version::PlatformVersion;

    #[test]
    fn test_no_epoch_change_same_protocol_version_succeeds() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();

        let epoch_info = EpochInfo::V0(EpochInfoV0 {
            current_epoch_index: 0,
            previous_epoch_index: None,
            is_epoch_change: false,
        });

        let block_info = BlockInfo {
            time_ms: 1_000_000,
            height: 2,
            core_height: 100,
            epoch: Epoch::default(),
        };

        let last_committed_state = platform.state.load();
        let mut block_platform_state = last_committed_state.as_ref().clone();

        let result = platform.upgrade_protocol_version_on_epoch_change_v0(
            &block_info,
            &epoch_info,
            &last_committed_state,
            &mut block_platform_state,
            &transaction,
            platform_version,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_not_epoch_change_with_different_protocol_version_returns_error() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();

        // Not an epoch change
        let epoch_info = EpochInfo::V0(EpochInfoV0 {
            current_epoch_index: 0,
            previous_epoch_index: None,
            is_epoch_change: false,
        });

        let block_info = BlockInfo {
            time_ms: 1_000_000,
            height: 2,
            core_height: 100,
            epoch: Epoch::default(),
        };

        let last_committed_state = platform.state.load();
        let mut block_platform_state = last_committed_state.as_ref().clone();

        // Use a different (lower) protocol version so previous != current
        // The platform was initialized with latest, now pass an older version
        let older_version = PlatformVersion::first();
        assert_ne!(
            older_version.protocol_version, platform_version.protocol_version,
            "this test requires at least two protocol versions to exist"
        );

        let result = platform.upgrade_protocol_version_on_epoch_change_v0(
            &block_info,
            &epoch_info,
            &last_committed_state,
            &mut block_platform_state,
            &transaction,
            older_version,
        );

        assert!(result.is_err());
        match result {
            Err(Error::Execution(ExecutionError::CorruptedCodeExecution(msg))) => {
                assert!(msg.contains("unexpected protocol upgrade"));
            }
            _ => panic!("expected CorruptedCodeExecution error"),
        }
    }

    #[test]
    fn test_epoch_change_blocks_global_cache() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();

        // Simulate epoch change (not genesis)
        let epoch_info = EpochInfo::V0(EpochInfoV0 {
            current_epoch_index: 5,
            previous_epoch_index: Some(4),
            is_epoch_change: true,
        });

        let block_info = BlockInfo {
            time_ms: 1_000_000,
            height: 500,
            core_height: 100,
            epoch: Epoch::new(5).expect("expected epoch"),
        };

        let last_committed_state = platform.state.load();
        let mut block_platform_state = last_committed_state.as_ref().clone();

        // Ensure the global cache starts unblocked
        {
            let counter = platform.drive.cache.protocol_versions_counter.read();
            assert!(counter.get(&platform_version.protocol_version).is_ok());
        }

        let result = platform.upgrade_protocol_version_on_epoch_change_v0(
            &block_info,
            &epoch_info,
            &last_committed_state,
            &mut block_platform_state,
            &transaction,
            platform_version,
        );

        assert!(result.is_ok());

        // After epoch change, the global cache should be blocked
        {
            let counter = platform.drive.cache.protocol_versions_counter.read();
            assert!(counter.get(&platform_version.protocol_version).is_err());
        }
    }

    #[test]
    fn test_epoch_change_inserts_fee_version_when_empty() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();

        let epoch_info = EpochInfo::V0(EpochInfoV0 {
            current_epoch_index: 1,
            previous_epoch_index: Some(0),
            is_epoch_change: true,
        });

        let block_info = BlockInfo {
            time_ms: 1_000_000,
            height: 100,
            core_height: 100,
            epoch: Epoch::new(1).expect("expected epoch"),
        };

        let last_committed_state = platform.state.load();
        let mut block_platform_state = last_committed_state.as_ref().clone();

        // Clear fee versions to test the "empty" path
        block_platform_state.previous_fee_versions_mut().clear();

        let result = platform.upgrade_protocol_version_on_epoch_change_v0(
            &block_info,
            &epoch_info,
            &last_committed_state,
            &mut block_platform_state,
            &transaction,
            platform_version,
        );

        assert!(result.is_ok());

        // Verify that a fee version was inserted for the current epoch
        let fee_versions = block_platform_state.previous_fee_versions();
        assert!(
            fee_versions.get(&1u16).is_some(),
            "expected fee version to be inserted for epoch 1"
        );
    }

    #[test]
    fn test_epoch_change_with_upgrade_vote_sets_next_version() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();

        // Use the current version as the "next" vote target, so it doesn't
        // exceed the latest supported version (which would trigger a warning
        // path that requires genesis_time to be stored).
        let voted_version = platform_version.protocol_version;

        // Insert enough votes for the current version
        {
            let mut counter = platform.drive.cache.protocol_versions_counter.write();
            counter.global_cache.insert(voted_version, 100);
        }

        let epoch_info = EpochInfo::V0(EpochInfoV0 {
            current_epoch_index: 2,
            previous_epoch_index: Some(1),
            is_epoch_change: true,
        });

        let block_info = BlockInfo {
            time_ms: 1_000_000,
            height: 200,
            core_height: 100,
            epoch: Epoch::new(2).expect("expected epoch"),
        };

        let last_committed_state = platform.state.load();
        let mut block_platform_state = last_committed_state.as_ref().clone();

        let result = platform.upgrade_protocol_version_on_epoch_change_v0(
            &block_info,
            &epoch_info,
            &last_committed_state,
            &mut block_platform_state,
            &transaction,
            platform_version,
        );

        assert!(result.is_ok(), "expected ok, got error: {:?}", result.err());

        // The next epoch protocol version should now be set to the voted version
        assert_eq!(
            block_platform_state.next_epoch_protocol_version(),
            voted_version
        );
    }

    #[test]
    fn test_genesis_epoch_change_does_not_trigger_upgrade_logic() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();

        // Genesis epoch change (epoch 0, is_epoch_change = true)
        // is_epoch_change_but_not_genesis() returns false for epoch 0
        let epoch_info = EpochInfo::V0(EpochInfoV0 {
            current_epoch_index: 0,
            previous_epoch_index: None,
            is_epoch_change: true,
        });

        let block_info = BlockInfo {
            time_ms: 1_000_000,
            height: 1,
            core_height: 100,
            epoch: Epoch::default(),
        };

        let last_committed_state = platform.state.load();
        let mut block_platform_state = last_committed_state.as_ref().clone();

        // Global cache should still be accessible after genesis epoch (no blocking)
        let result = platform.upgrade_protocol_version_on_epoch_change_v0(
            &block_info,
            &epoch_info,
            &last_committed_state,
            &mut block_platform_state,
            &transaction,
            platform_version,
        );

        assert!(result.is_ok());

        // The global cache should NOT be blocked (genesis epoch doesn't trigger upgrade)
        {
            let counter = platform.drive.cache.protocol_versions_counter.read();
            assert!(counter.get(&platform_version.protocol_version).is_ok());
        }
    }
}
