mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use grovedb::TransactionArg;

impl Drive {
    /// The masternodes that voted on a yes/no poll: the yes voters, then the no voters, then
    /// the abstaining ones.
    ///
    /// # Parameters
    /// - `vote_poll`: the poll.
    /// - `transaction`: the transaction to read in.
    /// - `platform_version`: the platform version.
    ///
    /// # Returns
    /// The voters' pro tx hashes.
    pub fn fetch_identities_voting_in_yes_no_vote_poll(
        &self,
        vote_poll: &YesNoVotePoll,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Identifier>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .yes_no
            .fetch_identities_voting_in_yes_no_vote_poll
        {
            Some(0) => self.fetch_identities_voting_in_yes_no_vote_poll_v0(
                vote_poll,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identities_voting_in_yes_no_vote_poll".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_identities_voting_in_yes_no_vote_poll".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
