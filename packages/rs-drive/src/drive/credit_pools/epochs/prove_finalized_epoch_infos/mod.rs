mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use grovedb::TransactionArg;

use dpp::version::PlatformVersion;

impl Drive {
    /// Prove finalized epoch information for a given range of epochs
    pub fn prove_finalized_epoch_infos(
        &self,
        start_epoch_index: u16,
        start_epoch_index_included: bool,
        end_epoch_index: u16,
        end_epoch_index_included: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .credit_pools
            .epochs
            .prove_finalized_epoch_infos
        {
            0 => self.prove_finalized_epoch_infos_v0(
                start_epoch_index,
                start_epoch_index_included,
                end_epoch_index,
                end_epoch_index_included,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_finalized_epoch_infos".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
