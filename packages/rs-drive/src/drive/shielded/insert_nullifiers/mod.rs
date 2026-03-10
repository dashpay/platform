mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Inserts nullifiers into the shielded pool.
    ///
    /// This performs two operations:
    /// 1. Inserts each nullifier into the permanent nullifiers tree (double-spend prevention)
    /// 2. Stores all nullifiers into per-block sync storage (for catch-up RPCs)
    ///
    /// # Parameters
    /// - `nullifiers`: The 32-byte nullifiers to insert
    /// - `block_height`: The current block height (for sync storage)
    /// - `block_time_ms`: The current block time in milliseconds (for sync storage expiration)
    /// - `transaction`: The database transaction
    /// - `platform_version`: The platform version for dispatch
    pub fn insert_nullifiers(
        &self,
        nullifiers: &[[u8; 32]],
        block_height: u64,
        block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version.drive.methods.shielded.insert_nullifiers {
            0 => self.insert_nullifiers_v0(
                nullifiers,
                block_height,
                block_time_ms,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_nullifiers".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
