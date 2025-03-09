use crate::errors::consensus::state::state_error::StateError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Identity cannot contain more than {max_items} public keys")]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct MaxIdentityPublicKeyLimitReachedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub max_items: usize,
}

impl MaxIdentityPublicKeyLimitReachedError {
    pub fn new(max_items: usize) -> Self {
        Self { max_items }
    }

    pub fn max_items(&self) -> usize {
        self.max_items
    }
}
impl From<MaxIdentityPublicKeyLimitReachedError> for ConsensusError {
    fn from(err: MaxIdentityPublicKeyLimitReachedError) -> Self {
        Self::StateError(StateError::MaxIdentityPublicKeyLimitReachedError(err))
    }
}
