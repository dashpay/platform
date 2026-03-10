mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

use crate::drive::shielded::nullifiers::types::NullifierChangePerBlock;

impl Drive {
    /// Fetches recent nullifier changes starting from a given block height.
    pub fn fetch_recent_nullifier_changes(
        &self,
        start_height: u64,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<NullifierChangePerBlock>, Error> {
        match platform_version
            .drive
            .methods
            .saved_block_transactions
            .fetch_nullifiers
        {
            0 => self.fetch_recent_nullifier_changes_v0(
                start_height,
                limit,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_recent_nullifier_changes".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
