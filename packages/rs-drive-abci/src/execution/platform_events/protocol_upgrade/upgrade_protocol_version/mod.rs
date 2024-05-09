mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::epoch_info::EpochInfo;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C> {
    /// Sets current protocol version and next epoch protocol version to block platform state
    ///
    /// This function should be called on very top of bock production before we add new proposed version for the next epoch
    ///
    /// It takes five parameters:
    /// * `block_info`: Information about the current block.
    /// * `epoch_info`: Information about the current epoch.
    /// * `last_committed_platform_state`: The last committed state of the platform.
    /// * `block_platform_state`: The current state of the platform.
    /// * `transaction`: The current transaction.
    /// * `platform_version`: The current version of the platform.
    ///
    /// # Errors
    ///
    /// This function will return an error if the previous block protocol version does not match the current block protocol version not on epoch change
    pub fn upgrade_protocol_version_on_epoch_change(
        &self,
        block_info: &BlockInfo,
        epoch_info: &EpochInfo,
        last_committed_platform_state: &PlatformState,
        block_platform_state: &mut PlatformState,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .protocol_upgrade
            .upgrade_protocol_version_on_epoch_change
        {
            0 => self.upgrade_protocol_version_on_epoch_change_v0(
                block_info,
                epoch_info,
                last_committed_platform_state,
                block_platform_state,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "upgrade_protocol_version_on_epoch_change".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
