mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::identity::masternode_vote::v0::PreviousVoteCount;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use grovedb::TransactionArg;

impl Drive {
    /// Registers a masternode's vote on an identity contender vote poll and applies the
    /// operations. The vote was validated by rs-drive-abci: the poll is in its vote phase, the
    /// choice is a contender or abstain, and the masternode may still change its vote.
    #[allow(clippy::too_many_arguments)]
    pub fn register_identity_contender_vote_poll_identity_vote(
        &self,
        voter_pro_tx_hash: [u8; 32],
        strength: u8,
        vote_poll: IdentityContenderVotePoll,
        vote_choice: ResourceVoteChoice,
        previous_resource_vote_choice_to_remove: Option<(ResourceVoteChoice, PreviousVoteCount)>,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .identity_contender
            .register_identity_contender_vote_poll_identity_vote
        {
            0 => self.register_identity_contender_vote_poll_identity_vote_v0(
                voter_pro_tx_hash,
                strength,
                vote_poll,
                vote_choice,
                previous_resource_vote_choice_to_remove,
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "register_identity_contender_vote_poll_identity_vote".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// The operations registering a masternode's vote on an identity contender vote poll.
    #[allow(clippy::too_many_arguments)]
    pub fn register_identity_contender_vote_poll_identity_vote_operations(
        &self,
        voter_pro_tx_hash: [u8; 32],
        strength: u8,
        vote_poll: IdentityContenderVotePoll,
        vote_choice: ResourceVoteChoice,
        previous_resource_vote_choice_to_remove: Option<(ResourceVoteChoice, PreviousVoteCount)>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .identity_contender
            .register_identity_contender_vote_poll_identity_vote
        {
            0 => self.register_identity_contender_vote_poll_identity_vote_operations_v0(
                voter_pro_tx_hash,
                strength,
                vote_poll,
                vote_choice,
                previous_resource_vote_choice_to_remove,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "register_identity_contender_vote_poll_identity_vote_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
