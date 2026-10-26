mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::platform_value::Bytes36;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves the committed state of one asset-lock outpoint, including absence.
    pub fn prove_asset_lock_outpoint_info(
        &self,
        outpoint: &Bytes36,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .asset_lock
            .prove_asset_lock_outpoint_info
        {
            0 => self.prove_asset_lock_outpoint_info_v0(outpoint, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_asset_lock_outpoint_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
