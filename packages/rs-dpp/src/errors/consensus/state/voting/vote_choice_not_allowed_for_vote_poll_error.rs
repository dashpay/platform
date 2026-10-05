use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use crate::voting::vote_polls::VotePoll;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A vote choice the vote poll does not offer: a contested index resolved by
/// `MasternodeVoteNoLocking` has no Lock choice.
#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error("VotePoll {vote_poll} does not allow the vote choice {vote_choice}")]
#[platform_serialize(unversioned)]
pub struct VoteChoiceNotAllowedForVotePollError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    vote_poll: VotePoll,
    vote_choice: ResourceVoteChoice,
}

impl VoteChoiceNotAllowedForVotePollError {
    pub fn new(vote_poll: VotePoll, vote_choice: ResourceVoteChoice) -> Self {
        Self {
            vote_poll,
            vote_choice,
        }
    }

    pub fn vote_poll(&self) -> &VotePoll {
        &self.vote_poll
    }

    pub fn vote_choice(&self) -> ResourceVoteChoice {
        self.vote_choice
    }
}

impl From<VoteChoiceNotAllowedForVotePollError> for ConsensusError {
    fn from(err: VoteChoiceNotAllowedForVotePollError) -> Self {
        Self::StateError(StateError::VoteChoiceNotAllowedForVotePollError(err))
    }
}
