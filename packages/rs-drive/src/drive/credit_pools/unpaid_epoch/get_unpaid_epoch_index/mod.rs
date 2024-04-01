use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::epoch::EpochIndex;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

mod v0;

impl Drive {
    /// Returns the index of the unpaid Epoch.
    pub fn get_unpaid_epoch_index(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<EpochIndex, Error> {
        match platform_version
            .drive
            .methods
            .credit_pools
            .unpaid_epoch
            .get_unpaid_epoch_index
        {
            0 => self.get_unpaid_epoch_index_v0(transaction),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_unpaid_epoch_index".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
