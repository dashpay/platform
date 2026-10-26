mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::asset_lock::StoredAssetLockInfo;
use dpp::platform_value::Bytes36;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies a proof of one asset-lock outpoint, including a not-present result.
    pub fn verify_asset_lock_outpoint_info(
        proof: &[u8],
        outpoint: &Bytes36,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, StoredAssetLockInfo), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .asset_lock
            .verify_asset_lock_outpoint_info
        {
            0 => Self::verify_asset_lock_outpoint_info_v0(proof, outpoint, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_asset_lock_outpoint_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
