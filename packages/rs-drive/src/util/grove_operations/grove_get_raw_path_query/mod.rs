mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::drive_versions::DriveVersion;

use grovedb::query_result_type::{QueryResultElements, QueryResultType};
use grovedb::{PathQuery, TransactionArg};

impl Drive {
    /// Retrieves the raw results of a path query from GroveDB.
    ///
    /// # Parameters
    /// * `path_query`: The path query to execute.
    /// * `transaction`: The GroveDB transaction associated with this operation.
    /// * `result_type`: The desired result type of the query.
    /// * `drive_operations`: A vector to collect the costs of operations for later computation.
    /// * `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok((QueryResultElements, u16))` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the platform version does not match known versions.
    /// * `Err(DriveError::GroveDB)` if the GroveDB operation returned an error.
    pub fn grove_get_raw_path_query(
        &self,
        path_query: &PathQuery,
        transaction: TransactionArg,
        result_type: QueryResultType,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(QueryResultElements, u16), Error> {
        match drive_version.grove_methods.basic.grove_get_raw_path_query {
            0 => self.grove_get_raw_path_query_v0(
                path_query,
                transaction,
                result_type,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "grove_get_raw_path_query".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
