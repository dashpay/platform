mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;

impl Drive {
    /// Removes the votes of finished yes/no polls with the three sum trees that hold them. The
    /// polls' stored info stays as the record of the decision.
    pub fn remove_yes_no_vote_poll_votes_operations(
        &self,
        vote_polls: &[&YesNoVotePoll],
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
            0 => self.remove_yes_no_vote_poll_votes_operations_v0(
                vote_polls,
                batch_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_yes_no_vote_poll_votes_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
