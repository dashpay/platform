mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::block::epoch::Epoch;

use grovedb::TransactionArg;

use dpp::version::PlatformVersion;

impl Drive {
    /// Returns the block height of the Epoch's start block
    ///
    /// # Arguments
    ///
    /// * `epoch_tree` - An Epoch instance.
    /// * `transaction` - A TransactionArg instance.
    /// * `platform_version` - A PlatformVersion instance representing the version of Platform.
    ///
    /// # Returns
    ///
    /// A Result containing the start block height or an Error.
    pub fn get_epoch_start_block_height(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<u64, Error> {
        match platform_version
            .drive
            .methods
            .credit_pools
            .epochs
            .get_epoch_start_block_height
        {
            0 => self.get_epoch_start_block_height_v0(epoch_tree, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_epoch_start_block_height".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
