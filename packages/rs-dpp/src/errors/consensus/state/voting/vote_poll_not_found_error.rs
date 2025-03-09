use crate::errors::consensus::state::state_error::StateError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::voting::vote_polls::VotePoll;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("VotePoll {vote_poll} not found")]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct VotePollNotFoundError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub vote_poll: VotePoll,
}

impl VotePollNotFoundError {
    pub fn new(vote_poll: VotePoll) -> Self {
        Self { vote_poll }
    }

    pub fn vote_poll(&self) -> &VotePoll {
        &self.vote_poll
    }
}

impl From<VotePollNotFoundError> for ConsensusError {
    fn from(err: VotePollNotFoundError) -> Self {
        Self::StateError(StateError::VotePollNotFoundError(err))
    }
}
