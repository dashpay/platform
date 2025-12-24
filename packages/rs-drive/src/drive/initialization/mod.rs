//! Drive Initialization

mod genesis_core_height;
mod v0;
mod v1;
mod v2;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Creates the initial state structure.
    pub fn create_initial_state_structure(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .initialization
            .create_initial_state_structure
        {
            0 => self.create_initial_state_structure_v0(transaction, platform_version),
            1 => self.create_initial_state_structure_v1(transaction, platform_version),
            2 => self.create_initial_state_structure_v2(transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "create_initial_state_structure".to_string(),
                known_versions: vec![0, 1, 2],
                received: version,
            })),
        }
    }
}
