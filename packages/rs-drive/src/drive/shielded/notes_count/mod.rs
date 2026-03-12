mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Returns the total number of notes in the shielded pool commitment tree.
    ///
    /// # Parameters
    /// - `transaction`: The GroveDB transaction
    /// - `drive_operations`: A vector to collect the costs of operations
    /// - `platform_version`: The platform version for dispatch
    ///
    /// # Returns
    /// The number of notes currently in the commitment tree.
    pub fn shielded_pool_notes_count(
        &self,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<u64, Error> {
        match platform_version.drive.methods.shielded.notes_count {
            0 => self.shielded_pool_notes_count_v0(transaction, drive_operations, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "shielded_pool_notes_count".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
