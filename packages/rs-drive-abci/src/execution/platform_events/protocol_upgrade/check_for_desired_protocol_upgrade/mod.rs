use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::PlatformVersion;

mod v0;
mod v1;

impl<C> Platform<C> {
    /// Checks for a network upgrade and resets activation window.
    /// This method should only be called when a new epoch starts.
    ///
    /// # Arguments
    ///
    /// * `active_hpmns` - The total number of evonodes that are not banned.
    ///
    /// # Returns
    ///
    /// * `Result<Option<ProtocolVersion>, Error>` - Returns the new protocol version if an upgrade was needed,
    ///   or None if no upgrade is required.
    ///   In case of an error, the corresponding Error is returned.
    ///
    /// # Errors
    ///
    /// This function may return an error if any of the following conditions are met:
    ///
    /// * There is an issue interacting with the underlying storage.
    /// * An overflow occurs when calculating the required block count.
    /// * More than one version pass the threshold to upgrade.
    pub fn check_for_desired_protocol_upgrade(
        &self,
        active_hpmns: u32,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ProtocolVersion>, Error> {
        match platform_version
            .drive_abci
            .methods
            .protocol_upgrade
            .check_for_desired_protocol_upgrade
        {
            0 => self.check_for_desired_protocol_upgrade_v0(active_hpmns, platform_version),
            1 => self.check_for_desired_protocol_upgrade_v1(active_hpmns, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "check_for_desired_protocol_upgrade".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
