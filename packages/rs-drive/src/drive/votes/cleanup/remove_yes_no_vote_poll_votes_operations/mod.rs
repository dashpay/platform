mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;

impl Drive {
    /// Removes the votes of finished yes/no polls with the three sum trees that hold them. The
    /// polls' stored info stays as the record of the decision.
    ///
    /// # Parameters
    /// - `vote_poll_ids`: the unique ids of the finished polls.
    /// - `batch_operations`: where the removals are added.
    /// - `platform_version`: the platform version.
    ///
    /// # Returns
    /// `Ok(())` once the removals are added.
    pub fn remove_yes_no_vote_poll_votes_operations(
        &self,
        vote_poll_ids: &[Identifier],
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .yes_no
            .remove_yes_no_vote_poll_votes_operations
        {
            Some(0) => self.remove_yes_no_vote_poll_votes_operations_v0(
                vote_poll_ids,
                batch_operations,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_yes_no_vote_poll_votes_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "remove_yes_no_vote_poll_votes_operations".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
