use crate::drive::votes::resolved::vote_polls::ResolvedVotePoll;
use crate::drive::votes::resolved::votes::resolved_resource_vote::accessors::v0::ResolvedResourceVoteGettersV0;
use crate::drive::votes::resolved::votes::ResolvedVote;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::identity::masternode_vote::v0::PreviousVoteCount;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Version 1 (protocol version 14) registers a vote of either kind: a resource vote as
    /// before, and a yes/no vote under its poll in the decisions branch.
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
            ResolvedVote::ResolvedResourceVote(resource_vote) => {
                let vote_choice = resource_vote.resource_vote_choice();
                match resource_vote.vote_poll_owned() {
                    ResolvedVotePoll::ContestedDocumentResourceVotePollWithContractInfo(
                        contested_document_resource_vote_poll,
                    ) => self.register_contested_resource_identity_vote(
                        voter_pro_tx_hash,
                        strength,
                        contested_document_resource_vote_poll,
                        vote_choice,
                        previous_resource_vote_choice_to_remove,
                        block_info,
                        transaction,
                        platform_version,
                    ),
                    ResolvedVotePoll::YesNoVotePoll(_) => {
                        Err(Error::Drive(DriveError::CorruptedCodeExecution(
                            "a resource vote cannot answer a yes/no vote poll",
                        )))
                    }
                }
            }
            ResolvedVote::YesNoVote(yes_no_vote) => self.register_yes_no_identity_vote(
                voter_pro_tx_hash,
                strength,
                yes_no_vote,
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
            ResolvedVote::ResolvedResourceVote(resource_vote) => {
                let vote_choice = resource_vote.resource_vote_choice();
                match resource_vote.vote_poll_owned() {
                    ResolvedVotePoll::ContestedDocumentResourceVotePollWithContractInfo(
                        contested_document_resource_vote_poll,
                    ) => self.register_contested_resource_identity_vote_operations(
                        voter_pro_tx_hash,
                        strength,
                        contested_document_resource_vote_poll,
                        vote_choice,
                        previous_resource_vote_choice_to_remove,
                        transaction,
                        platform_version,
                    ),
                    ResolvedVotePoll::YesNoVotePoll(_) => {
                        Err(Error::Drive(DriveError::CorruptedCodeExecution(
                            "a resource vote cannot answer a yes/no vote poll",
                        )))
                    }
                }
            }
            ResolvedVote::YesNoVote(yes_no_vote) => self.register_yes_no_identity_vote_operations(
                voter_pro_tx_hash,
                strength,
                yes_no_vote,
                transaction,
                platform_version,
            ),
        }
    }
}
