use crate::drive::votes::resolved::votes::ResolvedVote;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::identity::masternode_vote::v0::PreviousVoteCount;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Version 1 (protocol version 15) registers a vote of either kind: a yes/no vote under its
    /// poll in the decisions branch, and a resource vote through version 0, unchanged.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn register_identity_vote_v1(
        &self,
        voter_pro_tx_hash: [u8; 32],
        strength: u8,
        vote: ResolvedVote,
        previous_resource_vote_choice_to_remove: Option<(ResourceVoteChoice, PreviousVoteCount)>,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match vote {
            ResolvedVote::YesNoVote(yes_no_vote) => self.register_yes_no_identity_vote(
                voter_pro_tx_hash,
                strength,
                yes_no_vote,
                block_info,
                transaction,
                platform_version,
            ),
            resource_vote => self.register_identity_vote_v0(
                voter_pro_tx_hash,
                strength,
                resource_vote,
                previous_resource_vote_choice_to_remove,
                block_info,
                transaction,
                platform_version,
            ),
        }
    }

    pub(super) fn register_identity_vote_operations_v1(
        &self,
        voter_pro_tx_hash: [u8; 32],
        strength: u8,
        vote: ResolvedVote,
        previous_resource_vote_choice_to_remove: Option<(ResourceVoteChoice, PreviousVoteCount)>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match vote {
            ResolvedVote::YesNoVote(yes_no_vote) => self.register_yes_no_identity_vote_operations(
                voter_pro_tx_hash,
                strength,
                yes_no_vote,
                transaction,
                platform_version,
            ),
            resource_vote => self.register_identity_vote_operations_v0(
                voter_pro_tx_hash,
                strength,
                resource_vote,
                previous_resource_vote_choice_to_remove,
                transaction,
                platform_version,
            ),
        }
    }
}
