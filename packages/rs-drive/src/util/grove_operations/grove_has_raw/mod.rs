mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;

use dpp::version::drive_versions::DriveVersion;

use grovedb::TransactionArg;
use grovedb_path::SubtreePath;

impl Drive {
    /// Checks whether an element exists in groveDB at the specified path and key.
    /// The operation's cost is then appended to `drive_operations` for later processing.
    ///
    /// # Parameters
    /// * `path`: The groveDB hierarchical authenticated structure path where the element resides.
    /// * `key`: The key where the element resides within the subtree.
    /// * `query_type`: The type of query to perform, either `StatelessDirectQuery` or `StatefulDirectQuery`.
    /// * `transaction`: The groveDB transaction associated with this operation.
    /// * `drive_operations`: A vector to collect the costs of operations for later computation.
    /// * `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(bool)` if the operation was successful, returning `true` if the element exists and `false` otherwise.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the platform version does not match known versions.
    pub fn grove_has_raw<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        query_type: DirectQueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        match drive_version.grove_methods.basic.grove_has_raw {
            0 => self.grove_has_raw_v0(
                path,
                key,
                query_type,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "grove_has_raw".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
