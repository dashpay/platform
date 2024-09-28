mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::epoch_info::EpochInfo;
use crate::platform_types::platform::Platform;
use dpp::version::PlatformVersion;
use dpp::version::ProtocolVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C> {
    /// Executes specific events that need to be performed on the first block of a protocol change.
    ///
    /// # Parameters
    ///
    /// - `&self`: A reference to the current instance of the struct implementing this function.
    /// - `block_info`: Information about the current block, encapsulated in a `BlockInfo` struct.
    /// - `epoch_info`: Information about the current epoch, encapsulated in an `EpochInfo` struct.
    /// - `last_committed_platform_state`: The state of the platform as it was after the last committed block, before the protocol change.
    /// - `block_platform_state`: A mutable reference to the platform state that will be modified for the current block.
    /// - `transaction`: The transaction object representing the current transaction context.
    /// - `platform_version`: The version of the platform being executed, encapsulated in a `PlatformVersion` struct.
    ///
    /// # Returns
    ///
    /// - `Result<(), Error>`: Returns `Ok(())` if the events are successfully executed.
    ///   Returns an `Error` if there is a version mismatch or another execution issue.
    ///
    /// # Errors
    ///
    /// - Returns an `Error::Execution(ExecutionError::UnknownVersionMismatch)` if an unknown version is received
    ///   that is not supported by the current implementation.
    ///
    /// # Versioning
    ///
    /// This function uses the `platform_version` parameter to determine which version-specific implementation
    /// of the protocol change events should be executed:
    ///
    /// - If the version is `0`, it calls the `perform_events_on_first_block_of_protocol_change_v0` function,
    ///   which contains the logic for version `0`.
    /// - If no version is specified (`None`), the function does nothing and returns `Ok(())`.
    /// - If a different version is specified, it returns an error indicating an unknown version mismatch.
    ///
    pub fn perform_events_on_first_block_of_protocol_change(
        &self,
        epoch_info: &EpochInfo,
        transaction: &Transaction,
        previous_protocol_version: ProtocolVersion,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .protocol_upgrade
            .perform_events_on_first_block_of_protocol_change
        {
            Some(0) => self.perform_events_on_first_block_of_protocol_change_v0(
                epoch_info,
                transaction,
                previous_protocol_version,
                platform_version,
            ),
            None => return Ok(()),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "perform_events_on_first_block_of_protocol_change".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
