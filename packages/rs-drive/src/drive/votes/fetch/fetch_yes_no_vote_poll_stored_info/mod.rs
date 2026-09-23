mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::YesNoVotePollStoredInfo;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use grovedb::TransactionArg;

impl Drive {
    /// The stored info of a yes/no vote poll, `None` when the poll was never opened.
    pub fn fetch_yes_no_vote_poll_stored_info(
        &self,
        vote_poll: &YesNoVotePoll,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<YesNoVotePollStoredInfo>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .yes_no
            .fetch_yes_no_vote_poll_stored_info
        {
            0 => {
                self.fetch_yes_no_vote_poll_stored_info_v0(vote_poll, transaction, platform_version)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_yes_no_vote_poll_stored_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
