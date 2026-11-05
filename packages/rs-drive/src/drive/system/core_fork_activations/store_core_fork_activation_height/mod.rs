mod v0;

use crate::drive::system::core_fork_activations::CoreFork;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::prelude::CoreBlockHeight;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Records `core_height` as the chain locked Core height of the first Platform block that
    /// saw `fork` active. The record is written once and never changed: recording a fork again
    /// is an error. The write is applied at once, so later reads in the transaction see it.
    ///
    /// # Parameters
    /// - `fork`: the Core fork.
    /// - `core_height`: the chain locked Core height of the block that saw it active.
    /// - `transaction`: the transaction to write in; in a block, the block's transaction.
    /// - `platform_version`: the platform version.
    ///
    /// # Returns
    /// `Ok(())` once the height is written, `CorruptedCodeExecution` when the fork is already
    /// recorded.
    pub fn store_core_fork_activation_height(
        &self,
        fork: CoreFork,
        core_height: CoreBlockHeight,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .platform_system
            .store_core_fork_activation_height
        {
            Some(0) => self.store_core_fork_activation_height_v0(
                fork,
                core_height,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "store_core_fork_activation_height".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "store_core_fork_activation_height".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
