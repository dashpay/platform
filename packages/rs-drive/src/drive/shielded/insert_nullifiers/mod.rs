mod v0;

use crate::drive::shielded::paths::token_shielded_pool_nullifiers_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Inserts nullifiers into the shielded pool's permanent nullifiers tree
    /// (double-spend prevention).
    ///
    /// # Parameters
    /// - `nullifiers`: The 32-byte nullifiers to insert
    /// - `platform_version`: The platform version for dispatch
    pub fn insert_nullifiers(
        &self,
        nullifiers: &[[u8; 32]],
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version.drive.methods.shielded.insert_nullifiers {
            0 => self.insert_nullifiers_v0(nullifiers),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_nullifiers".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl Drive {
    /// Inserts nullifiers into a TOKEN shielded pool's permanent nullifiers tree (double-spend
    /// prevention). Same versioning as [`Drive::insert_nullifiers`].
    pub fn insert_token_pool_nullifiers(
        token_id: [u8; 32],
        nullifiers: &[[u8; 32]],
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version.drive.methods.shielded.insert_nullifiers {
            0 => Ok(Self::insert_nullifiers_in_pool_v0(
                token_shielded_pool_nullifiers_path_vec(token_id),
                nullifiers,
            )),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_token_pool_nullifiers".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
