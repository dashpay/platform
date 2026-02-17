mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::version::drive_versions::DriveVersion;

use grovedb::PathQuery;

impl Drive {
    /// Generates a V1 proof for the specified path query in groveDB.
    /// V1 proofs support BulkAppendTree/CommitmentTree subqueries.
    /// The operation's cost is then added to `drive_operations` for later processing.
    ///
    /// # Parameters
    /// * `path_query`: The groveDB path query to generate a V1 proof for.
    /// * `drive_operations`: A vector to collect the costs of operations for later computation.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` if the operation was successful, returning the serialized proof bytes.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn grove_get_proved_path_query_v1(
        &self,
        path_query: &PathQuery,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        match drive_version
            .grove_methods
            .basic
            .grove_get_proved_path_query_v1
        {
            0 => self.grove_get_proved_path_query_v1_v0(
                path_query,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "grove_get_proved_path_query_v1".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
