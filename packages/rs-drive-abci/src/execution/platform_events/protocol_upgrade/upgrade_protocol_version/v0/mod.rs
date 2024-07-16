use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::epoch_info::v0::{EpochInfoV0Getters, EpochInfoV0Methods};
use crate::platform_types::epoch_info::EpochInfo;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use chrono::{TimeZone, Utc};
use dpp::block::block_info::BlockInfo;
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

            let hpmn_list_len = last_committed_platform_state.hpmn_list_len() as u32;

            let next_epoch_protocol_version =
                self.check_for_desired_protocol_upgrade(hpmn_list_len, platform_version)?;

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
                if *last_fee_version != platform_version.fee_version {
                    previous_fee_versions_map.insert(
                        epoch_info.current_epoch_index(),
                        platform_version.fee_version.clone(),
                    );
                }
            // In case of empty cached_fee_version, insert the new (epoch_index, fee_version)
            } else {
                previous_fee_versions_map.insert(
                    epoch_info.current_epoch_index(),
                    platform_version.fee_version.clone(),
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
