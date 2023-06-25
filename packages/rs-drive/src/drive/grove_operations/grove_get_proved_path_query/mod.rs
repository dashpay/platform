mod v0;

use grovedb::{PathQuery};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use grovedb::TransactionArg;
use dpp::version::drive_versions::DriveVersion;

impl Drive {
    pub fn grove_get_proved_path_query(
        &self,
        path_query: &PathQuery,
        verbose: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        match drive_version.grove_methods.basic.grove_get_proved_path_query {
            0 => self.grove_get_proved_path_query_v0(path_query, verbose, transaction, drive_operations),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "grove_get_proved_path_query".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}