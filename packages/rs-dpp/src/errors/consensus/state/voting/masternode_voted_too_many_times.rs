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
#[error("Masternode with id: {pro_tx_hash} voted {times_voted}, which is too many times")]
#[platform_serialize(unversioned)]
pub struct MasternodeVotedTooManyTimesError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pro_tx_hash: Identifier,

    times_voted: u16,
}

impl MasternodeVotedTooManyTimesError {
    pub fn new(pro_tx_hash: Identifier, times_voted: u16) -> Self {
        Self {
            pro_tx_hash,
            times_voted,
        }
    }

    pub fn pro_tx_hash(&self) -> Identifier {
        self.pro_tx_hash
    }
}

impl From<MasternodeVotedTooManyTimesError> for ConsensusError {
    fn from(err: MasternodeVotedTooManyTimesError) -> Self {
        Self::StateError(StateError::MasternodeVotedTooManyTimesError(err))
    }
}
