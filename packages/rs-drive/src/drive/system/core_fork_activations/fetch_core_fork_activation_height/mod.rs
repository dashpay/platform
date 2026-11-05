mod v0;

use crate::drive::system::core_fork_activations::CoreFork;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::prelude::CoreBlockHeight;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches the chain locked Core height of the first Platform block that saw `fork` active.
    ///
    /// Read in a block after its Core info is updated, `Some` means the fork is active at the
    /// block's chain locked height. `None` means Platform has not seen it active, not that it
    /// is inactive: a block that keeps the chain locked height does not look.
    ///
    /// # Parameters
    /// - `fork`: the Core fork.
    /// - `transaction`: the transaction to read in; in a block, the block's transaction.
    /// - `platform_version`: the platform version.
    ///
    /// # Returns
    /// The recorded height, or `None` when nothing is recorded.
    pub fn fetch_core_fork_activation_height(
        &self,
        fork: CoreFork,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<CoreBlockHeight>, Error> {
        match platform_version
            .drive
            .methods
            .platform_system
            .fetch_core_fork_activation_height
        {
            Some(0) => {
                self.fetch_core_fork_activation_height_v0(fork, transaction, platform_version)
            }
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_core_fork_activation_height".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_core_fork_activation_height".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
