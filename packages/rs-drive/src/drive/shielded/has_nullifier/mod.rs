mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Checks whether a nullifier has already been spent in the shielded pool.
    ///
    /// Nullifiers are stored in the nullifiers tree at
    /// `[AddressBalances, "s", [2]]`. Uses O(1) key lookup.
    ///
    /// # Parameters
    /// - `nullifier`: The 32-byte nullifier to look up
    /// - `transaction`: The GroveDB transaction
    /// - `drive_operations`: A vector to collect the costs of operations
    /// - `platform_version`: The platform version for dispatch
    ///
    /// # Returns
    /// `Ok(true)` if the nullifier exists (already spent), `Ok(false)` otherwise.
    pub fn has_nullifier(
        &self,
        nullifier: &[u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        match platform_version.drive.methods.shielded.has_nullifier {
            0 => self.has_nullifier_v0(nullifier, transaction, drive_operations, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "has_nullifier".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
