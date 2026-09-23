mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    /// Removes the votes of finished yes/no polls and the three sum trees that held them. The
    /// polls' stored info stays as the record of the decision.
    pub fn remove_yes_no_vote_poll_votes_operations(
        &self,
        vote_polls: &[(
            &YesNoVotePoll,
            &BTreeMap<YesNoAbstainVoteChoice, Vec<Identifier>>,
        )],
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
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
                transaction,
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
