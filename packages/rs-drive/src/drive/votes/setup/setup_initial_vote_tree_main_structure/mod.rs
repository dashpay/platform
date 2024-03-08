mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::version::PlatformVersion;
use crate::drive::batch::GroveDbOpBatch;

impl Drive {
    pub fn add_initial_vote_tree_main_structure_operations(
        batch: &mut GroveDbOpBatch,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .setup
            .add_initial_vote_tree_main_structure_operations
        {
            0 => Drive::add_initial_vote_tree_main_structure_operations_v0(batch),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_initial_vote_tree_main_structure_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
