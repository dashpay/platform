mod v0;

use crate::block::block_info::BlockInfo;
use crate::voting::contender_structs::{
    ContenderWithSerializedDocument, FinalizedResourceVoteChoicesWithVoterInfo,
};
use crate::voting::vote_info_storage::contested_document_vote_poll_stored_info::v0::ContestedDocumentVotePollStoredInfoV0;
use crate::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use crate::ProtocolError;
use bincode::{Decode, Encode};
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::{PlatformDeserializable, PlatformSerializable};

    fn sb(time: u64, height: u64) -> BlockInfo {
        BlockInfo {
            time_ms: time,
            height,
            ..BlockInfo::default()
        }
    }

    #[test]
    fn new_uses_latest_supported_version() {
        let pv = PlatformVersion::latest();
        let info = ContestedDocumentVotePollStoredInfo::new(sb(1, 1), pv)
            .expect("should construct for latest version");
        // At present only V0 exists
        match info {
            ContestedDocumentVotePollStoredInfo::V0(v0) => {
                assert!(matches!(
                    v0.vote_poll_status,
                    ContestedDocumentVotePollStatus::Started(_)
                ));
            }
        }
    }

    #[test]
    fn update_to_latest_version_is_idempotent_for_v0() {
        let pv = PlatformVersion::latest();
        let info = ContestedDocumentVotePollStoredInfo::new(sb(1, 1), pv).unwrap();
        let updated = info
            .clone()
            .update_to_latest_version(pv)
            .expect("should be ok");
        assert_eq!(info, updated);
    }

    #[test]
    fn finalize_routes_through_wrapper() {
        let pv = PlatformVersion::latest();
        let mut info = ContestedDocumentVotePollStoredInfo::new(sb(1, 1), pv).unwrap();
        info.finalize_vote_poll(
            vec![],
            sb(2, 2),
            ContestedDocumentVotePollWinnerInfo::Locked,
        )
        .expect("should finalize");
        // After locked winner, status should be Locked.
        assert!(matches!(
            info.vote_poll_status(),
            ContestedDocumentVotePollStatus::Locked
        ));
    }

    #[test]
    fn display_contains_wrapper_name() {
        let pv = PlatformVersion::latest();
        let info = ContestedDocumentVotePollStoredInfo::new(sb(1, 1), pv).unwrap();
        let rendered = info.to_string();
        assert!(rendered.starts_with("V0("));
    }

    #[test]
    fn serialization_roundtrip() {
        let pv = PlatformVersion::latest();
        let info = ContestedDocumentVotePollStoredInfo::new(sb(1, 1), pv).unwrap();
        let bytes = info.serialize_to_bytes().expect("serialize");
        let restored = ContestedDocumentVotePollStoredInfo::deserialize_from_bytes(&bytes)
            .expect("deserialize");
        assert_eq!(info, restored);
    }

    #[test]
    fn getters_proxy_to_inner_v0() {
        let pv = PlatformVersion::latest();
        let mut info = ContestedDocumentVotePollStoredInfo::new(sb(1, 1), pv).unwrap();
        // Before any finalize, many getters return None or defaults
        assert!(info.last_resource_vote_choices().is_none());
        assert!(info.awarded_block().is_none());
        assert_eq!(info.current_start_block(), Some(sb(1, 1)));
        assert!(info.last_finalization_block().is_none());
        assert_eq!(info.winner(), ContestedDocumentVotePollWinnerInfo::NoWinner);
        assert!(info.last_locked_votes().is_none());
        assert!(info.last_locked_voters().is_none());
        assert!(info.last_abstain_votes().is_none());
        assert!(info.last_abstain_voters().is_none());
        assert!(info
            .contender_votes_in_vec_of_contender_with_serialized_document()
            .is_none());
        assert!(matches!(
            info.vote_poll_status_ref(),
            ContestedDocumentVotePollStatus::Started(_)
        ));

        // After an identity-winning finalize, awarded_block / winner populated.
        let id = Identifier::new([7u8; 32]);
        info.finalize_vote_poll(
            vec![],
            sb(2, 2),
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(id),
        )
        .unwrap();
        assert_eq!(info.awarded_block(), Some(sb(2, 2)));
        assert_eq!(
            info.winner(),
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(id)
        );
        assert_eq!(info.last_finalization_block(), Some(sb(2, 2)));
    }
}
