use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::{Bytes20, Identifier};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Masternode {pro_tx_hash} voting address is incorrect, current is {current_voting_address}, given is {given_voting_address}")]
#[platform_serialize(unversioned)]
pub struct MasternodeIncorrectVotingAddressError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pro_tx_hash: Identifier,

    current_voting_address: Bytes20,

    given_voting_address: Bytes20,
}

impl MasternodeIncorrectVotingAddressError {
    pub fn new(
        pro_tx_hash: Identifier,
        current_voting_address: Bytes20,
        given_voting_address: Bytes20,
    ) -> Self {
        Self {
            pro_tx_hash,
            current_voting_address,
            given_voting_address,
        }
    }

    pub fn pro_tx_hash(&self) -> Identifier {
        self.pro_tx_hash
    }

    pub fn current_voting_address(&self) -> Bytes20 {
        self.current_voting_address
    }

    pub fn given_voting_address(&self) -> Bytes20 {
        self.given_voting_address
    }
}

impl From<MasternodeIncorrectVotingAddressError> for ConsensusError {
    fn from(err: MasternodeIncorrectVotingAddressError) -> Self {
        Self::StateError(StateError::MasternodeIncorrectVotingAddressError(err))
    }
}
