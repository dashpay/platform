use crate::errors::consensus::state::state_error::StateError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Masternode {pro_tx_hash} not found")]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct MasternodeNotFoundError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub pro_tx_hash: Identifier,
}

impl MasternodeNotFoundError {
    pub fn new(pro_tx_hash: Identifier) -> Self {
        Self { pro_tx_hash }
    }

    pub fn pro_tx_hash(&self) -> Identifier {
        self.pro_tx_hash
    }
}

impl From<MasternodeNotFoundError> for ConsensusError {
    fn from(err: MasternodeNotFoundError) -> Self {
        Self::StateError(StateError::MasternodeNotFoundError(err))
    }
}
