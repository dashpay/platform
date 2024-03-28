use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
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
    pub(super) fn upgrade_protocol_version_v0(
        &self,
        last_committed_platform_state: &PlatformState,
        block_platform_state: &mut PlatformState,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Store current protocol version in drive state
        // TODO: This will be removed in #1778
        self.drive.store_current_protocol_version(
            platform_version.protocol_version,
            Some(transaction),
            &platform_version.drive,
        )?;

        // Determine a new protocol version for the next epoch if enough proposers voted
        // otherwise keep the current one

        let hpmn_list_len = last_committed_platform_state.hpmn_list_len() as u32;

        let next_epoch_protocol_version =
            self.check_for_desired_protocol_upgrade(hpmn_list_len, platform_version)?;

        if let Some(protocol_version) = next_epoch_protocol_version {
            block_platform_state.set_next_epoch_protocol_version(protocol_version);
        }

        // Since we are starting a new epoch we need to drop previously
        // proposed versions

        // Remove previously proposed versions from Drive state
        self.drive
            .clear_version_information(Some(transaction), &platform_version.drive)
            .map_err(Error::Drive)?;

        // We clean voting counter cache only on finalize block because:
        // 1. The voting counter global cache uses for querying of voting information in Drive queries
        // 2. There might be multiple rounds so on the next round we will lose all previous epoch votes
        //
        // Instead of clearing cache, the further block processing logic is using `get_if_enabled`
        // to get a version counter from the global cache. We disable this getter here to prevent
        // reading previous voting information for the new epoch.
        // The getter must be enabled back on finalize block in [update_drive_cache] and at the very beginning
        // of the block processing in [clear_drive_block_cache].

        let mut protocol_versions_counter = self.drive.cache.protocol_versions_counter.write();
        protocol_versions_counter.disable_counter_getter();
        drop(protocol_versions_counter);

        Ok(())
    }
}
