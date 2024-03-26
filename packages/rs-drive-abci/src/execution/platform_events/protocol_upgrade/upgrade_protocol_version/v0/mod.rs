use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::epoch_info::v0::{EpochInfoV0Getters, EpochInfoV0Methods};
use crate::platform_types::epoch_info::EpochInfo;
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
        epoch_info: &EpochInfo,
        last_committed_platform_state: &PlatformState,
        block_platform_state: &mut PlatformState,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let current_block_protocol_version = platform_version.protocol_version;
        let previous_block_protocol_version =
            last_committed_platform_state.current_protocol_version_in_consensus();

        // Set current protocol version
        block_platform_state
            .set_current_protocol_version_in_consensus(current_block_protocol_version);

        // Protocol version upgrade should happen only on epoch change
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

                // We need to persist new protocol version
                // TODO: Must be a part of epoch trees. The matter of the next PR
                self.drive.store_current_protocol_version(
                    current_block_protocol_version,
                    Some(transaction),
                    &platform_version.drive,
                )?;
            };

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
            self.drive
                .clear_version_information(Some(transaction), &platform_version.drive)
                .map_err(Error::Drive)?;
        } else if previous_block_protocol_version != current_block_protocol_version {
            return Err(Error::Execution(
                ExecutionError::UnexpectedProtocolVersionUpgrade,
            ));
        }

        Ok(())
    }
}
