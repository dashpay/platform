mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::platform_value::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    /// The masternodes that voted for each of the given contenders of an identity contender
    /// vote poll, and for abstaining when asked. A choice nobody voted for has no entry.
    pub fn fetch_identities_voting_in_identity_contender_vote_poll(
        &self,
        vote_poll: &IdentityContenderVotePoll,
        contenders: Vec<Identifier>,
        also_fetch_abstaining_votes: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<ResourceVoteChoice, Vec<Identifier>>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .identity_contender
            .fetch_identities_voting_in_identity_contender_vote_poll
        {
            0 => self.fetch_identities_voting_in_identity_contender_vote_poll_v0(
                vote_poll,
                contenders,
                also_fetch_abstaining_votes,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identities_voting_in_identity_contender_vote_poll".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
