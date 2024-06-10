use crate::drive::votes::resolved::vote_polls::ResolvedVotePoll;
use crate::drive::votes::resolved::votes::resolved_resource_vote::accessors::v0::ResolvedResourceVoteGettersV0;
use crate::drive::votes::resolved::votes::ResolvedVote;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

impl Drive {
    pub(super) fn register_identity_vote_v0(
        &self,
        voter_pro_tx_hash: [u8; 32],
        strength: u8,
        vote: ResolvedVote,
        previous_resource_vote_choice_to_remove: Option<ResourceVoteChoice>,
        block_info: &BlockInfo,
        apply: bool,
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
                        apply,
                        transaction,
                        platform_version,
                    ),
                }
            }
        }
    }

    pub(super) fn register_identity_vote_operations_v0(
        &self,
        voter_pro_tx_hash: [u8; 32],
        strength: u8,
        vote: ResolvedVote,
        previous_resource_vote_choice_to_remove: Option<ResourceVoteChoice>,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
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
                        block_info,
                        estimated_costs_only_with_layer_info,
                        transaction,
                        platform_version,
                    ),
                }
            }
        }
    }
}
