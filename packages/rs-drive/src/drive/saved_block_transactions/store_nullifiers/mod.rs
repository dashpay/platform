mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Stores nullifiers for a block in the SavedBlockTransactions tree.
    ///
    /// This method serializes the nullifiers using bincode and stores
    /// them keyed by block height. If compaction thresholds are exceeded, it will
    /// compact existing entries along with the current block's data and store
    /// an expiration time for the compacted entry.
    ///
    /// # Parameters
    /// - `nullifiers`: The nullifiers to store for this block
    /// - `block_height`: The height of the block these nullifiers belong to
    /// - `block_time_ms`: The block time in milliseconds (used for expiration calculation)
    /// - `transaction`: The database transaction
    /// - `platform_version`: The platform version
    ///
    /// # Returns
    /// - `Ok(())` on success
    /// - `Err(Error)` if the operation fails
    pub fn store_nullifiers_for_block(
        &self,
        nullifiers: &[[u8; 32]],
        block_height: u64,
        block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .saved_block_transactions
            .store_nullifiers
        {
            0 => self.store_nullifiers_for_block_v0(
                nullifiers,
                block_height,
                block_time_ms,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "store_nullifiers_for_block".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
