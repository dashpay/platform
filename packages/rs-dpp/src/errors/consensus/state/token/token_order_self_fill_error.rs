use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// An identity cannot fill its own order: it cancels it instead.
#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error("Identity {identity_id} cannot fill its own token order {order_id}")]
#[platform_serialize(unversioned)]
pub struct TokenOrderSelfFillError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    order_id: Identifier,
    identity_id: Identifier,
}

impl TokenOrderSelfFillError {
    pub fn new(order_id: Identifier, identity_id: Identifier) -> Self {
        Self {
            order_id,
            identity_id,
        }
    }

    pub fn order_id(&self) -> Identifier {
        self.order_id
    }

    pub fn identity_id(&self) -> Identifier {
        self.identity_id
    }
}

impl From<TokenOrderSelfFillError> for ConsensusError {
    fn from(err: TokenOrderSelfFillError) -> Self {
        Self::StateError(StateError::TokenOrderSelfFillError(err))
    }
}
