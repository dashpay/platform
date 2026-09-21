use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::voting::vote_info_storage::identity_contender_vote_poll_stored_info::IdentityContenderVotePollStatus;
use crate::voting::vote_polls::VotePoll;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A vote on an identity contender poll outside its vote phase: while contenders still join,
/// or after it resolved.
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
#[error("Identity contender VotePoll {vote_poll} is not available for voting: {status}")]
#[platform_serialize(unversioned)]
pub struct IdentityContenderVotePollNotAvailableForVotingError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    vote_poll: VotePoll,
    status: IdentityContenderVotePollStatus,
}

impl IdentityContenderVotePollNotAvailableForVotingError {
    pub fn new(vote_poll: VotePoll, status: IdentityContenderVotePollStatus) -> Self {
        Self { vote_poll, status }
    }

    pub fn vote_poll(&self) -> &VotePoll {
        &self.vote_poll
    }

    pub fn status(&self) -> IdentityContenderVotePollStatus {
        self.status
    }
}

impl From<IdentityContenderVotePollNotAvailableForVotingError> for ConsensusError {
    fn from(err: IdentityContenderVotePollNotAvailableForVotingError) -> Self {
        Self::StateError(StateError::IdentityContenderVotePollNotAvailableForVotingError(err))
    }
}
