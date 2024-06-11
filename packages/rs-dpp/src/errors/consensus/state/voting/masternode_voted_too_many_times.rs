use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Masternode with id: {pro_tx_hash} already voted {times_already_voted} times and is trying to vote again, they can only vote {max_times_allowed} times")]
#[platform_serialize(unversioned)]
pub struct MasternodeVotedTooManyTimesError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pro_tx_hash: Identifier,

    times_already_voted: u16,

    max_times_allowed: u16,
}

impl MasternodeVotedTooManyTimesError {
    pub fn new(pro_tx_hash: Identifier, times_already_voted: u16, max_times_allowed: u16) -> Self {
        Self {
            pro_tx_hash,
            times_already_voted,
            max_times_allowed,
        }
    }

    pub fn pro_tx_hash(&self) -> Identifier {
        self.pro_tx_hash
    }

    pub fn times_already_voted(&self) -> u16 {
        self.times_already_voted
    }

    pub fn max_times_allowed(&self) -> u16 {
        self.max_times_allowed
    }
}

impl From<MasternodeVotedTooManyTimesError> for ConsensusError {
    fn from(err: MasternodeVotedTooManyTimesError) -> Self {
        Self::StateError(StateError::MasternodeVotedTooManyTimesError(err))
    }
}
