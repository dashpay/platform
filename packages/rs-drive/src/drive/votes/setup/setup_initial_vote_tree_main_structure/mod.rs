mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    pub fn setup_initial_vote_tree_main_structure(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .setup_initial_vote_tree_main_structure
        {
            0 => self.setup_initial_vote_tree_main_structure_v0(transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "setup_initial_vote_tree_main_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
