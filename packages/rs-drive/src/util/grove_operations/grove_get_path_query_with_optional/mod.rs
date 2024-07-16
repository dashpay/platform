mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::drive_versions::DriveVersion;
use grovedb::query_result_type::PathKeyOptionalElementTrio;
use grovedb::PathQuery;
use grovedb::TransactionArg;

impl Drive {
    /// Retrieves the results of a path query from GroveDB with optional elements.
    ///
    /// # Parameters
    /// * `path_query`: The path query to execute.
    /// * `transaction`: The groveDB transaction associated with this operation.
    /// * `drive_operations`: A vector to collect the costs of operations for later computation.
    /// * `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(Vec<PathKeyOptionalElementTrio>)` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the platform version does not match known versions.
    /// * `Err(DriveError::GroveDB)` if the GroveDB operation returned an error.
    pub fn grove_get_path_query_with_optional(
        &self,
        path_query: &PathQuery,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Vec<PathKeyOptionalElementTrio>, Error> {
        match drive_version
            .grove_methods
            .basic
            .grove_get_path_query_with_optional
        {
            0 => self.grove_get_path_query_with_optional_v0(
                path_query,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "grove_get_path_query_with_optional".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
