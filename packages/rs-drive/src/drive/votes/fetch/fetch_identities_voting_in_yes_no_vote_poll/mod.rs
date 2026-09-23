mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    /// The masternodes that voted on a yes/no poll, by their answer. Every choice is present,
    /// with an empty list when nobody chose it.
    pub fn fetch_identities_voting_in_yes_no_vote_poll(
        &self,
        vote_poll: &YesNoVotePoll,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<YesNoAbstainVoteChoice, Vec<Identifier>>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .yes_no
            .fetch_identities_voting_in_yes_no_vote_poll
        {
            0 => self.fetch_identities_voting_in_yes_no_vote_poll_v0(
                vote_poll,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identities_voting_in_yes_no_vote_poll".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
