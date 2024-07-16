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
    /// Retrieves a u64 value from GroveDB that was originally encoded as a varint.
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
    /// * `Ok(Some(u64))` if the operation was successful and the element was found and could be decoded to a u64.
    /// * `Ok(None)` if the operation was successful but the element was not found.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the platform version does not match known versions.
    /// * `Err(DriveError::CorruptedElementType)` if the element was found but could not be decoded to a u64.
    /// * `Err(DriveError::CorruptedQueryReturnedNonItem)` if the query returned an unexpected non-item element.
    pub fn grove_get_raw_value_u64_from_encoded_var_vec<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        direct_query_type: DirectQueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Option<u64>, Error> {
        match drive_version
            .grove_methods
            .basic
            .grove_get_raw_value_u64_from_encoded_var_vec
        {
            0 => self.grove_get_raw_value_u64_from_encoded_var_vec_v0(
                path,
                key,
                direct_query_type,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "grove_get_raw_value_u64_from_encoded_var_vec".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
