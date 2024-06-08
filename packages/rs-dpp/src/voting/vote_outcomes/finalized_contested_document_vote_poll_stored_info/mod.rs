mod v0;

use crate::block::block_info::BlockInfo;
use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use crate::voting::contender_structs::{
    ContenderWithSerializedDocument, FinalizedResourceVoteChoicesWithVoterInfo,
};
use crate::voting::vote_outcomes::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use crate::voting::vote_outcomes::finalized_contested_document_vote_poll_stored_info::v0::FinalizedContestedDocumentVotePollStoredInfoV0;
use crate::ProtocolError;
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
pub use v0::FinalizedContestedDocumentVotePollStoredInfoV0Getters;

/// Represents the stored info after a contested document vote poll.
///
/// This struct holds the list of contenders, the abstaining vote tally.
#[derive(
    Debug, PartialEq, Eq, Clone, From, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[platform_serialize(unversioned)]
pub enum FinalizedContestedDocumentVotePollStoredInfo {
    /// V0.
    V0(FinalizedContestedDocumentVotePollStoredInfoV0),
}

impl FinalizedContestedDocumentVotePollStoredInfo {
    pub fn new(
        resource_vote_choices: Vec<FinalizedResourceVoteChoicesWithVoterInfo>,
        finalization_block: BlockInfo,
        winner: ContestedDocumentVotePollWinnerInfo,
        platform_version: &PlatformVersion,
    ) -> Result<FinalizedContestedDocumentVotePollStoredInfo, ProtocolError> {
        match platform_version
            .dpp
            .voting_versions
            .finalized_contested_document_vote_poll_stored_info_version
        {
            0 => Ok(FinalizedContestedDocumentVotePollStoredInfoV0::new(
                resource_vote_choices,
                finalization_block,
                winner,
            )
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "FinalizedContestedDocumentVotePollStoredInfo::new".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl FinalizedContestedDocumentVotePollStoredInfoV0Getters
    for FinalizedContestedDocumentVotePollStoredInfo
{
    fn resource_vote_choices(&self) -> &Vec<FinalizedResourceVoteChoicesWithVoterInfo> {
        match self {
            FinalizedContestedDocumentVotePollStoredInfo::V0(v0) => &v0.resource_vote_choices,
        }
    }

    fn finalization_block(&self) -> &BlockInfo {
        match self {
            FinalizedContestedDocumentVotePollStoredInfo::V0(v0) => &v0.finalization_block,
        }
    }

    fn winner(&self) -> &ContestedDocumentVotePollWinnerInfo {
        match self {
            FinalizedContestedDocumentVotePollStoredInfo::V0(v0) => &v0.winner,
        }
    }

    fn locked_votes(&self) -> u32 {
        match self {
            FinalizedContestedDocumentVotePollStoredInfo::V0(v0) => v0.locked_votes(),
        }
    }

    fn locked_voters(&self) -> Vec<Identifier> {
        match self {
            FinalizedContestedDocumentVotePollStoredInfo::V0(v0) => v0.locked_voters(),
        }
    }

    fn abstain_votes(&self) -> u32 {
        match self {
            FinalizedContestedDocumentVotePollStoredInfo::V0(v0) => v0.abstain_votes(),
        }
    }

    fn abstain_voters(&self) -> Vec<Identifier> {
        match self {
            FinalizedContestedDocumentVotePollStoredInfo::V0(v0) => v0.abstain_voters(),
        }
    }

    fn contender_votes_in_vec_of_contender_with_serialized_document(
        &self,
    ) -> Vec<ContenderWithSerializedDocument> {
        match self {
            FinalizedContestedDocumentVotePollStoredInfo::V0(v0) => {
                v0.contender_votes_in_vec_of_contender_with_serialized_document()
            }
        }
    }
}
