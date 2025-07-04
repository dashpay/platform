mod v0;

use grovedb::TransactionArg;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::block::epoch::Epoch;
use dpp::prelude::FeeMultiplier;
use dpp::version::PlatformVersion;

impl Drive {
    /// Gets the Fee Multiplier for the Epoch.
    ///
    /// # Arguments
    ///
    /// * `epoch_tree` - A reference to the Epoch.
    /// * `transaction` - A TransactionArg instance.
    /// * `platform_version` - A PlatformVersion instance representing the version of Platform.
    ///
    /// # Returns
    ///
    /// A Result containing either the fee multiplier for the epoch, if found,
    /// or an Error if something goes wrong.
    pub fn get_epoch_fee_multiplier(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeMultiplier, Error> {
        match platform_version
            .drive
            .methods
            .credit_pools
            .epochs
            .get_epoch_fee_multiplier
        {
            0 => self.get_epoch_fee_multiplier_v0(epoch_tree, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_epoch_fee_multiplier".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
