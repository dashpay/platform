use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::voting::vote_info_storage::yes_no_vote_poll_stored_info::YesNoVotePollStatus;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A vote on a yes/no poll that is not taking votes (protocol version 15): one never opened
/// (`status` is `None`), or one that is finished. A vote names its poll by id only, so the id
/// is all the error can name.
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
#[error(
    "yes/no vote poll {vote_poll_id} is not available for voting: {}",
    status_description(status)
)]
#[platform_serialize(unversioned)]
pub struct YesNoVotePollNotAvailableForVotingError {
    /*
    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION
    */
    vote_poll_id: Identifier,
    status: Option<YesNoVotePollStatus>,
}

fn status_description(status: &Option<YesNoVotePollStatus>) -> String {
    match status {
        None => "it was never opened".to_string(),
        Some(status) => status.to_string(),
    }
}

impl YesNoVotePollNotAvailableForVotingError {
    pub fn new(vote_poll_id: Identifier, status: Option<YesNoVotePollStatus>) -> Self {
        Self {
            vote_poll_id,
            status,
        }
    }

    pub fn vote_poll_id(&self) -> Identifier {
        self.vote_poll_id
    }

    /// Where the poll is in its life, `None` when it was never opened.
    pub fn status(&self) -> Option<&YesNoVotePollStatus> {
        self.status.as_ref()
    }
}

impl From<YesNoVotePollNotAvailableForVotingError> for ConsensusError {
    fn from(err: YesNoVotePollNotAvailableForVotingError) -> Self {
        Self::StateError(StateError::YesNoVotePollNotAvailableForVotingError(err))
    }
}
