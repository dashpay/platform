mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::epoch::Epoch;
use grovedb::TransactionArg;

use dpp::version::PlatformVersion;

impl Drive {
    /// Prove multiple epoch infos starting at a given epoch index
    pub fn prove_epochs_infos(
        &self,
        start_epoch_index: u16,
        count: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .credit_pools
            .epochs
            .prove_epochs_infos
        {
            0 => {
                self.prove_epochs_infos_v0(start_epoch_index, count, transaction, platform_version)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
