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
    /// Retrieves the total value from a sum tree within groveDB at the specified path and key.
    /// The cost of the operation is then appended to `drive_operations` for later processing.
    ///
    /// # Parameters
    /// * `path`: The groveDB hierarchical authenticated structure path where the sum tree is located.
    /// * `key`: The key where the sum tree resides within the subtree.
    /// * `query_type`: The type of query to perform, either `StatelessDirectQuery` or `StatefulDirectQuery`.
    /// * `transaction`: The groveDB transaction associated with this operation.
    /// * `drive_operations`: A vector to collect the costs of operations for later computation.
    /// * `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(i64)` if the operation was successful, returning the total value of the sum tree.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the platform version does not match known versions.
    /// * `Err(DriveError::CorruptedBalancePath)` if the balance path does not refer to a sum tree.
    /// * `Err(DriveError::CorruptedCodeExecution)` if trying to query a non-tree element.
    pub fn grove_get_sum_tree_total_value<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        query_type: DirectQueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<i64, Error> {
        match drive_version
            .grove_methods
            .basic
            .grove_get_sum_tree_total_value
        {
            0 => self.grove_get_sum_tree_total_value_v0(
                path,
                key,
                query_type,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "grove_get_sum_tree_total_value".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
