use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

mod v0;

impl<C> Platform<C> {
    /// Checks for a network upgrade and resets activation window.
    /// This method should only be called when a new epoch starts.
    /// It alters the underlying state but does not change the drive cache.
    ///
    /// # Arguments
    ///
    /// * `total_hpmns` - The total number of high priority masternodes.
    /// * `current_protocol_version_in_consensus` - The current protocol version in consensus.
    /// * `transaction` - A transaction argument to interact with the underlying storage.
    ///
    /// # Returns
    ///
    /// * `Result<Option<ProtocolVersion>, Error>` - Returns the new protocol version if an upgrade was needed,
    ///                                              or None if no upgrade is required.
    ///                                              In case of an error, the corresponding Error is returned.
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
        total_hpmns: u32,
        current_protocol_version_in_consensus: ProtocolVersion,
        transaction: &Transaction,
    ) -> Result<Option<ProtocolVersion>, Error> {
        let current_platform_version = PlatformVersion::get(current_protocol_version_in_consensus)?;
        match current_platform_version
            .drive_abci
            .methods
            .protocol_upgrade
            .check_for_desired_protocol_upgrade
        {
            0 => self.check_for_desired_protocol_upgrade_v0(
                total_hpmns,
                current_protocol_version_in_consensus,
                transaction,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "check_for_desired_protocol_upgrade".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
