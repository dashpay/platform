mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::identity_contender_vote_poll_stored_info::IdentityContenderVotePollStoredInfo;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use grovedb::TransactionArg;

impl Drive {
    /// The stored info of an identity contender vote poll: None for a poll that never opened,
    /// including when no such poll ever opened and the branch does not exist.
    pub fn fetch_identity_contender_vote_poll_stored_info(
        &self,
        vote_poll: &IdentityContenderVotePoll,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<IdentityContenderVotePollStoredInfo>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .identity_contender
            .fetch_identity_contender_vote_poll_stored_info
        {
            0 => self.fetch_identity_contender_vote_poll_stored_info_v0(
                vote_poll,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_contender_vote_poll_stored_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
