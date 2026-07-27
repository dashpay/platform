mod v0;

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
