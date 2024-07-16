mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;

use dpp::version::drive_versions::DriveVersion;

use grovedb::{Element, TransactionArg};
use grovedb_path::SubtreePath;

impl Drive {
    /// Handles the retrieval of a raw element from GroveDB at the specified path and key.
    /// The operation cost is added to `drive_operations` for later processing.
    ///
    /// # Parameters
    /// * `path`: The groveDB hierarchical authenticated structure path from where the element is to be retrieved.
    /// * `key`: The key of the element to be retrieved from the subtree.
    /// * `direct_query_type`: The type of query to perform, whether stateless or stateful.
    /// * `transaction`: The groveDB transaction associated with this operation.
    /// * `drive_operations`: A vector to collect the costs of operations for later computation.
    /// * `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(Some(Element))` if the operation was successful and the element was found.
    /// * `Ok(None)` if the operation was successful but the element was not found.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the platform version does not match known versions.
    pub fn grove_get_raw<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        direct_query_type: DirectQueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Option<Element>, Error> {
        match drive_version.grove_methods.basic.grove_get_raw {
            0 => self.grove_get_raw_v0(
                path,
                key,
                direct_query_type,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "grove_get_raw".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
