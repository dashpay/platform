mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::identity::masternode_vote::v0::PreviousVoteCount;
use dpp::platform_value::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use grovedb::TransactionArg;

impl Drive {
    /// The vote a masternode holds on an identity contender vote poll, with how many times it
    /// voted on the poll, or None when it never voted on it.
    pub fn fetch_identity_contender_vote_poll_identity_vote(
        &self,
        masternode_pro_tx_hash: Identifier,
        vote_poll_id: Identifier,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<(ResourceVoteChoice, PreviousVoteCount)>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .identity_contender
            .fetch_identity_contender_vote_poll_identity_vote
        {
            0 => self.fetch_identity_contender_vote_poll_identity_vote_v0(
                masternode_pro_tx_hash,
                vote_poll_id,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_contender_vote_poll_identity_vote".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
