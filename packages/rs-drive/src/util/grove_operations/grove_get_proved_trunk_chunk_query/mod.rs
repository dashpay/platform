//! Proved trunk chunk query in grove.

mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::version::drive_versions::DriveVersion;

use grovedb::PathTrunkChunkQuery;

impl Drive {
    /// Retrieves a proof of a trunk chunk query in groveDB.
    /// The operation's cost is then added to `drive_operations` for later processing.
    ///
    /// # Parameters
    /// * `query`: The trunk chunk query to retrieve a proof for.
    /// * `drive_operations`: A vector to collect the costs of operations for later computation.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` if the operation was successful, returning the serialized proof data.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn grove_get_proved_trunk_chunk_query(
        &self,
        query: &PathTrunkChunkQuery,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        match drive_version
            .grove_methods
            .basic
            .grove_get_proved_trunk_chunk_query
        {
            0 => self.grove_get_proved_trunk_chunk_query_v0(
                query,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "grove_get_proved_trunk_chunk_query".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
