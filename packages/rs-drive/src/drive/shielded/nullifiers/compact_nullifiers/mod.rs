mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

/// One week in milliseconds (used for compacted nullifier expiration)
pub const ONE_WEEK_IN_MS: u64 = 7 * 24 * 60 * 60 * 1000;

impl Drive {
    /// Compacts nullifiers from recent blocks, including the current block,
    /// into a single compacted entry.
    ///
    /// This function drains all entries from the nullifiers tree, concatenates them
    /// with the provided current block's nullifiers, and stores the result in
    /// the compacted nullifiers tree with a (start_block, end_block) key.
    ///
    /// Also stores the expiration time (current block time + 1 week) in the
    /// nullifiers expiration time tree.
    ///
    /// # Arguments
    /// * `current_nullifiers` - The current block's nullifiers to include
    /// * `current_block_height` - The height of the current block
    /// * `current_block_time_ms` - The current block time in milliseconds
    /// * `transaction` - Optional database transaction
    /// * `platform_version` - The platform version
    ///
    /// # Returns
    /// * `Ok((start, end))` - The block range that was compacted
    /// * `Err` - An error occurred
    pub fn compact_nullifiers_with_current_block(
        &self,
        current_nullifiers: &[[u8; 32]],
        current_block_height: u64,
        current_block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(u64, u64), Error> {
        match platform_version
            .drive
            .methods
            .saved_block_transactions
            .compact_nullifiers
        {
            0 => self.compact_nullifiers_with_current_block_v0(
                current_nullifiers,
                current_block_height,
                current_block_time_ms,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "compact_nullifiers_with_current_block".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
