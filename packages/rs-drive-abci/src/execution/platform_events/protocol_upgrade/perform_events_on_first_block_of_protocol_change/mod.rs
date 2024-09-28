mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::epoch_info::EpochInfo;
use crate::platform_types::platform::Platform;
use dpp::version::PlatformVersion;
use dpp::version::ProtocolVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C> {
    /// Executes protocol-specific events on the first block after a protocol version change.
    ///
    /// This function is triggered when there is a protocol version upgrade detected in the network.
    /// It checks if the current protocol version has transitioned from an earlier version to version 4,
    /// and if so, performs the necessary setup or migration tasks associated with version 4.
    ///
    /// Currently, the function handles the transition to version 4 by initializing new structures
    /// or states required for the new protocol version.
    ///
    /// # Parameters
    ///
    /// * `transaction`: A reference to the transaction context in which the changes should be applied.
    /// * `previous_protocol_version`: The protocol version prior to the upgrade.
    /// * `platform_version`: The current platform version containing the updated protocol version and relevant configuration details.
    ///
    /// # Returns
    ///
    /// * `Ok(())`: If all events related to the protocol change were successfully executed.
    /// * `Err(Error)`: If there was an issue executing the protocol-specific events.
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
