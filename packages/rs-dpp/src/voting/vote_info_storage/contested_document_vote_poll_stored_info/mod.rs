mod v0;

use crate::block::block_info::BlockInfo;
use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use crate::voting::contender_structs::{
    ContenderWithSerializedDocument, FinalizedResourceVoteChoicesWithVoterInfo,
};
use crate::voting::vote_info_storage::contested_document_vote_poll_stored_info::v0::ContestedDocumentVotePollStoredInfoV0;
use crate::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use crate::ProtocolError;
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use std::fmt;
pub use v0::ContestedDocumentVotePollStoredInfoV0Getters;

pub type LockedVotePollCounter = u16;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default, Encode, Decode)]
pub enum ContestedDocumentVotePollStatus {
    #[default]
    NotStarted,
    Awarded(Identifier),
    Locked,
    Started(BlockInfo),
}

impl fmt::Display for ContestedDocumentVotePollStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContestedDocumentVotePollStatus::NotStarted => write!(f, "NotStarted"),
            ContestedDocumentVotePollStatus::Awarded(identifier) => {
                write!(f, "Awarded({})", identifier)
            }
            ContestedDocumentVotePollStatus::Locked => write!(f, "Locked"),
            ContestedDocumentVotePollStatus::Started(block_info) => {
                write!(f, "Started({})", block_info)
            }
        }
    }
}

impl ContestedDocumentVotePollStatus {
    pub fn awarded_or_locked(&self) -> bool {
        matches!(
            self,
            ContestedDocumentVotePollStatus::Awarded(_) | ContestedDocumentVotePollStatus::Locked
        )
    }
}

/// Represents the stored info after a contested document vote poll.
///
/// This struct holds the list of contenders, the abstaining vote tally.
#[derive(
    Debug, PartialEq, Eq, Clone, From, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[platform_serialize(unversioned)]
pub enum ContestedDocumentVotePollStoredInfo {
    /// V0.
    V0(ContestedDocumentVotePollStoredInfoV0),
}

impl fmt::Display for ContestedDocumentVotePollStoredInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContestedDocumentVotePollStoredInfo::V0(info) => write!(f, "V0({})", info),
        }
    }
}

impl ContestedDocumentVotePollStoredInfo {
    pub fn new(
        start_block: BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<ContestedDocumentVotePollStoredInfo, ProtocolError> {
        match platform_version
            .dpp
            .voting_versions
            .contested_document_vote_poll_stored_info_version
        {
            0 => Ok(ContestedDocumentVotePollStoredInfoV0::new(start_block).into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ContestedDocumentVotePollStoredInfo::new".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    pub fn update_to_latest_version(
        self,
        platform_version: &PlatformVersion,
    ) -> Result<ContestedDocumentVotePollStoredInfo, ProtocolError> {
        match platform_version
            .dpp
            .voting_versions
            .contested_document_vote_poll_stored_info_version
        {
            0 => {
                // Nothing to do
                match self {
                    ContestedDocumentVotePollStoredInfo::V0(_) => Ok(self),
                }
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "FinalizedContestedDocumentVotePollStoredInfo::update_to_latest_version"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    pub fn finalize_vote_poll(
        &mut self,
        resource_vote_choices: Vec<FinalizedResourceVoteChoicesWithVoterInfo>,
        finalization_block: BlockInfo,
        winner: ContestedDocumentVotePollWinnerInfo,
    ) -> Result<(), ProtocolError> {
        match self {
            ContestedDocumentVotePollStoredInfo::V0(v0) => {
                v0.finalize_vote_poll(resource_vote_choices, finalization_block, winner)
            }
        }
    }
}

impl ContestedDocumentVotePollStoredInfoV0Getters for ContestedDocumentVotePollStoredInfo {
    fn last_resource_vote_choices(
        &self,
    ) -> Option<&Vec<FinalizedResourceVoteChoicesWithVoterInfo>> {
        match self {
            ContestedDocumentVotePollStoredInfo::V0(v0) => v0.last_resource_vote_choices(),
        }
    }

    fn awarded_block(&self) -> Option<BlockInfo> {
        match self {
            ContestedDocumentVotePollStoredInfo::V0(v0) => v0.awarded_block(),
        }
    }

    fn current_start_block(&self) -> Option<BlockInfo> {
        match self {
            ContestedDocumentVotePollStoredInfo::V0(v0) => v0.current_start_block(),
        }
    }

    fn last_finalization_block(&self) -> Option<BlockInfo> {
        match self {
            ContestedDocumentVotePollStoredInfo::V0(v0) => v0.last_finalization_block(),
        }
    }

    fn winner(&self) -> ContestedDocumentVotePollWinnerInfo {
        match self {
            ContestedDocumentVotePollStoredInfo::V0(v0) => v0.winner(),
        }
    }

    fn last_locked_votes(&self) -> Option<u32> {
        match self {
            ContestedDocumentVotePollStoredInfo::V0(v0) => v0.last_locked_votes(),
        }
    }

    fn last_locked_voters(&self) -> Option<Vec<(Identifier, u8)>> {
        match self {
            ContestedDocumentVotePollStoredInfo::V0(v0) => v0.last_locked_voters(),
        }
    }

    fn last_abstain_votes(&self) -> Option<u32> {
        match self {
            ContestedDocumentVotePollStoredInfo::V0(v0) => v0.last_abstain_votes(),
        }
    }

    fn last_abstain_voters(&self) -> Option<Vec<(Identifier, u8)>> {
        match self {
            ContestedDocumentVotePollStoredInfo::V0(v0) => v0.last_abstain_voters(),
        }
    }

    fn contender_votes_in_vec_of_contender_with_serialized_document(
        &self,
    ) -> Option<Vec<ContenderWithSerializedDocument>> {
        match self {
            ContestedDocumentVotePollStoredInfo::V0(v0) => {
                v0.contender_votes_in_vec_of_contender_with_serialized_document()
            }
        }
    }

    fn vote_poll_status(&self) -> ContestedDocumentVotePollStatus {
        match self {
            ContestedDocumentVotePollStoredInfo::V0(v0) => v0.vote_poll_status,
        }
    }

    fn vote_poll_status_ref(&self) -> &ContestedDocumentVotePollStatus {
        match self {
            ContestedDocumentVotePollStoredInfo::V0(v0) => &v0.vote_poll_status,
        }
    }
}
