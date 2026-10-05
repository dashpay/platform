use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// An `addedModerator` of the moderation charters contract for a seated charter that already
/// has as many additions as its target contract's elected declaration allows
/// (`maxAddedModerators`). The additions that exist count: deleting one frees its slot.
#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error(
    "Elected charter {} already has the {} added moderators contract {} allows",
    elected_charter_id,
    max_added_moderators,
    target_contract_id
)]
#[platform_serialize(unversioned)]
pub struct ModerationCharterAddedModeratorLimitReachedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    elected_charter_id: Identifier,
    target_contract_id: Identifier,
    max_added_moderators: u16,
}

impl ModerationCharterAddedModeratorLimitReachedError {
    pub fn new(
        elected_charter_id: Identifier,
        target_contract_id: Identifier,
        max_added_moderators: u16,
    ) -> Self {
        Self {
            elected_charter_id,
            target_contract_id,
            max_added_moderators,
        }
    }

    pub fn elected_charter_id(&self) -> Identifier {
        self.elected_charter_id
    }

    pub fn target_contract_id(&self) -> Identifier {
        self.target_contract_id
    }

    pub fn max_added_moderators(&self) -> u16 {
        self.max_added_moderators
    }
}

impl From<ModerationCharterAddedModeratorLimitReachedError> for ConsensusError {
    fn from(err: ModerationCharterAddedModeratorLimitReachedError) -> Self {
        Self::StateError(StateError::ModerationCharterAddedModeratorLimitReachedError(err))
    }
}
