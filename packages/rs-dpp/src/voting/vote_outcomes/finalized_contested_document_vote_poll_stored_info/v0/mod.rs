use crate::block::block_info::BlockInfo;
use crate::voting::contender_structs::{
    ContenderWithSerializedDocument, FinalizedResourceVoteChoicesWithVoterInfo,
};
use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use crate::voting::vote_outcomes::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use crate::voting::vote_outcomes::finalized_contested_document_vote_poll_stored_info::FinalizedContestedDocumentVotePollStoredInfo;
use bincode::{Decode, Encode};
use platform_value::Identifier;

#[derive(Debug, PartialEq, Eq, Clone, Default, Encode, Decode)]
pub struct FinalizedContestedDocumentVotePollStoredInfoV0 {
    /// The list of contenders returned by the query.
    pub resource_vote_choices: Vec<FinalizedResourceVoteChoicesWithVoterInfo>,
    /// Finalization Block
    pub finalization_block: BlockInfo,
    /// Winner info
    pub winner: ContestedDocumentVotePollWinnerInfo,
}

impl FinalizedContestedDocumentVotePollStoredInfoV0 {
    pub fn new(
        resource_vote_choices: Vec<FinalizedResourceVoteChoicesWithVoterInfo>,
        finalization_block: BlockInfo,
        winner: ContestedDocumentVotePollWinnerInfo,
    ) -> FinalizedContestedDocumentVotePollStoredInfoV0 {
        FinalizedContestedDocumentVotePollStoredInfoV0 {
            resource_vote_choices,
            finalization_block,
            winner,
        }
    }
}

pub trait FinalizedContestedDocumentVotePollStoredInfoV0Getters {
    fn resource_vote_choices(&self) -> &Vec<FinalizedResourceVoteChoicesWithVoterInfo>;
    fn finalization_block(&self) -> &BlockInfo;
    fn winner(&self) -> &ContestedDocumentVotePollWinnerInfo;

    fn locked_votes(&self) -> u32;

    fn locked_voters(&self) -> Vec<Identifier>;

    fn abstain_votes(&self) -> u32;

    fn abstain_voters(&self) -> Vec<Identifier>;
    fn contender_votes_in_vec_of_contender_with_serialized_document(
        &self,
    ) -> Vec<ContenderWithSerializedDocument>;
}

impl FinalizedContestedDocumentVotePollStoredInfoV0Getters
    for FinalizedContestedDocumentVotePollStoredInfoV0
{
    fn resource_vote_choices(&self) -> &Vec<FinalizedResourceVoteChoicesWithVoterInfo> {
        &self.resource_vote_choices
    }

    fn finalization_block(&self) -> &BlockInfo {
        &self.finalization_block
    }

    fn winner(&self) -> &ContestedDocumentVotePollWinnerInfo {
        &self.winner
    }

    fn locked_votes(&self) -> u32 {
        self.resource_vote_choices
            .iter()
            .filter(|choice| matches!(choice.resource_vote_choice, ResourceVoteChoice::Lock))
            .map(|choice| choice.voters.len() as u32)
            .sum()
    }

    fn locked_voters(&self) -> Vec<Identifier> {
        self.resource_vote_choices
            .iter()
            .filter(|choice| matches!(choice.resource_vote_choice, ResourceVoteChoice::Lock))
            .flat_map(|choice| choice.voters.clone())
            .collect()
    }

    fn abstain_votes(&self) -> u32 {
        self.resource_vote_choices
            .iter()
            .filter(|choice| matches!(choice.resource_vote_choice, ResourceVoteChoice::Abstain))
            .map(|choice| choice.voters.len() as u32)
            .sum()
    }

    fn abstain_voters(&self) -> Vec<Identifier> {
        self.resource_vote_choices
            .iter()
            .filter(|choice| matches!(choice.resource_vote_choice, ResourceVoteChoice::Abstain))
            .flat_map(|choice| choice.voters.clone())
            .collect()
    }

    fn contender_votes_in_vec_of_contender_with_serialized_document(
        &self,
    ) -> Vec<ContenderWithSerializedDocument> {
        self.resource_vote_choices
            .iter()
            .filter_map(|choice| {
                if let ResourceVoteChoice::TowardsIdentity(identity_id) =
                    &choice.resource_vote_choice
                {
                    Some(ContenderWithSerializedDocument {
                        identity_id: *identity_id,
                        serialized_document: None,
                        vote_tally: Some(choice.voters.len() as u32),
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}
