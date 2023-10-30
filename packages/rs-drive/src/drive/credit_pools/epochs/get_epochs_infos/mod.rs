mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::epoch::Epoch;
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use grovedb::TransactionArg;

use dpp::version::PlatformVersion;

impl Drive {
    /// Get multiple epoch infos starting at a given epoch index
    pub fn get_epochs_infos(
        &self,
        start_epoch_index: u16,
        count: u16,
        ascending: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<ExtendedEpochInfo>, Error> {
        match platform_version
            .drive
            .methods
            .credit_pools
            .epochs
            .get_epochs_infos
        {
            0 => self.get_epochs_infos_v0(
                start_epoch_index,
                count,
                ascending,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_epochs_infos".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
