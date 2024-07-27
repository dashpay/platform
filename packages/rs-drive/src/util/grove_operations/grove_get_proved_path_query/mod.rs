mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::version::drive_versions::DriveVersion;

use grovedb::{PathQuery, TransactionArg};

impl Drive {
    /// Retrieves a proof of the specified path query in groveDB.
    /// The operation's cost is then added to `drive_operations` for later processing.
    ///
    /// # Parameters
    /// * `path_query`: The groveDB path query to retrieve a proof for.
    /// * `verbose`: Specifies whether to include all paths in the proof (when set to `true`) or only those that were
    ///   affected by changes (when set to `false`).
    /// * `transaction`: The groveDB transaction associated with this operation.
    /// * `drive_operations`: A vector to collect the costs of operations for later computation.
    /// * `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` if the operation was successful, returning the proof data.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the platform version does not match known versions.
    pub fn grove_get_proved_path_query(
        &self,
        path_query: &PathQuery,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        match drive_version
            .grove_methods
            .basic
            .grove_get_proved_path_query
        {
            0 => self.grove_get_proved_path_query_v0(
                path_query,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "grove_get_proved_path_query".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
