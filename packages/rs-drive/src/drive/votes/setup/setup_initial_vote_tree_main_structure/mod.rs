mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;

use crate::util::batch::GroveDbOpBatch;
use dpp::version::PlatformVersion;

impl Drive {
    /// Initializes the main structure of the vote tree within a GroveDB operation batch.
    /// This function is version-controlled to ensure compatibility with different versions of the platform.
    ///
    /// # Parameters
    ///
    /// - `batch`: A mutable reference to a GroveDbOpBatch, which will accumulate the necessary operations
    ///   to set up the main vote tree structure.
    /// - `platform_version`: A reference to the platform version to ensure the correct setup operations
    ///   are applied based on the specified version.
    ///
    /// # Returns
    ///
    /// Returns a `Result<(), Error>`, indicating successful setup of the initial vote tree structure
    /// within the provided batch or an error in case of failure.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    /// - The platform version is unknown or unsupported, resulting in a version mismatch error.
    /// - Specific operations for the given version fail to be added to the batch, potentially due to
    ///   constraints or issues within the GroveDB operation batch.
    ///
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
