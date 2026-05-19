mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::version::drive_versions::DriveVersion;

use grovedb::TransactionArg;
use grovedb_path::SubtreePath;

impl Drive {
    /// Returns the total number of items in a CommitmentTree.
    /// The operation's cost is then added to `drive_operations` for later processing.
    ///
    /// # Parameters
    /// * `path`: The path to the CommitmentTree's parent.
    /// * `key`: The key of the CommitmentTree element.
    /// * `transaction`: The groveDB transaction associated with this operation.
    /// * `drive_operations`: A vector to collect the costs of operations for later computation.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(u64)` — the total number of items in the CommitmentTree.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn grove_commitment_tree_count<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<u64, Error> {
        match drive_version
            .grove_methods
            .basic
            .grove_commitment_tree_count
        {
            0 => self.grove_commitment_tree_count_v0(
                path,
                key,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "grove_commitment_tree_count".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
