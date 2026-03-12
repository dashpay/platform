mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Checks whether a shielded pool anchor exists in the anchors tree.
    ///
    /// Anchors are stored as `anchor_bytes -> block_height_be` in
    /// `[AddressBalances, "s", [6]]`. Uses O(1) key lookup.
    ///
    /// # Parameters
    /// - `anchor`: The 32-byte anchor to look up
    /// - `transaction`: The GroveDB transaction
    /// - `drive_operations`: A vector to collect the costs of operations
    /// - `platform_version`: The platform version for dispatch
    ///
    /// # Returns
    /// `Ok(true)` if the anchor exists, `Ok(false)` otherwise.
    pub fn has_shielded_anchor(
        &self,
        anchor: &[u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        match platform_version.drive.methods.shielded.has_anchor {
            0 => {
                self.has_shielded_anchor_v0(anchor, transaction, drive_operations, platform_version)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "has_shielded_anchor".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
