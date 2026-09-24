use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::voting::vote_info_storage::yes_no_vote_poll_stored_info::YesNoVotePollStatus;
use crate::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A vote on a yes/no poll that exists but is no longer taking votes (protocol version 14).
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
#[error("yes/no vote poll {vote_poll} is not available for voting: {status}")]
#[platform_serialize(unversioned)]
pub struct YesNoVotePollNotAvailableForVotingError {
    /*
    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION
    */
    vote_poll: YesNoVotePoll,
    status: YesNoVotePollStatus,
}

impl YesNoVotePollNotAvailableForVotingError {
    pub fn new(vote_poll: YesNoVotePoll, status: YesNoVotePollStatus) -> Self {
        Self { vote_poll, status }
    }

    pub fn vote_poll(&self) -> &YesNoVotePoll {
        &self.vote_poll
    }

    pub fn status(&self) -> &YesNoVotePollStatus {
        &self.status
    }
}

impl From<YesNoVotePollNotAvailableForVotingError> for ConsensusError {
    fn from(err: YesNoVotePollNotAvailableForVotingError) -> Self {
        Self::StateError(StateError::YesNoVotePollNotAvailableForVotingError(err))
    }
}
