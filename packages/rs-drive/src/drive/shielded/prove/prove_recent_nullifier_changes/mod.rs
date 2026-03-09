//! Proves recent nullifier changes starting from a given block height.

mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Proves recent nullifier changes starting from a given block height.
    ///
    /// # Arguments
    /// * `start_height` - The block height to start from
    /// * `limit` - Optional maximum number of blocks to prove
    /// * `transaction` - Optional database transaction
    /// * `platform_version` - The platform version
    ///
    /// # Returns
    /// A grovedb proof
    pub fn prove_recent_nullifier_changes(
        &self,
        start_height: u64,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .saved_block_transactions
            .fetch_nullifiers
        {
            0 => self.prove_recent_nullifier_changes_v0(
                start_height,
                limit,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_recent_nullifier_changes".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
