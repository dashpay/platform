mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    /// The operations removing what an identity contender vote poll held while it ran: every
    /// vote, every contender with its record, and the abstain tree. `votes` names every choice
    /// of the poll with its voters, including the choices nobody voted for, so that every
    /// tree goes. The stored info stays, with the result.
    pub fn remove_identity_contender_vote_poll_operations(
        &self,
        vote_poll: &IdentityContenderVotePoll,
        votes: &BTreeMap<ResourceVoteChoice, Vec<Identifier>>,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .identity_contender
            .remove_identity_contender_vote_poll_operations
        {
            0 => self.remove_identity_contender_vote_poll_operations_v0(
                vote_poll,
                votes,
                batch_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_identity_contender_vote_poll_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
