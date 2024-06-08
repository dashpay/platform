use crate::block::block_info::BlockInfo;
use crate::voting::contender_structs::FinalizedResourceVoteChoicesWithVoterInfo;
use crate::voting::vote_outcomes::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use bincode::{Decode, Encode};

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
