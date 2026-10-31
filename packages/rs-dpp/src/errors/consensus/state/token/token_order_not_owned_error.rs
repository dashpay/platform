use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// Only an order's maker can cancel it.
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
#[error("Token order {order_id} belongs to {owner_id}, not {sender_id}")]
#[platform_serialize(unversioned)]
pub struct TokenOrderNotOwnedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    order_id: Identifier,
    owner_id: Identifier,
    sender_id: Identifier,
}

impl TokenOrderNotOwnedError {
    pub fn new(order_id: Identifier, owner_id: Identifier, sender_id: Identifier) -> Self {
        Self {
            order_id,
            owner_id,
            sender_id,
        }
    }

    pub fn order_id(&self) -> Identifier {
        self.order_id
    }

    pub fn owner_id(&self) -> Identifier {
        self.owner_id
    }

    pub fn sender_id(&self) -> Identifier {
        self.sender_id
    }
}

impl From<TokenOrderNotOwnedError> for ConsensusError {
    fn from(err: TokenOrderNotOwnedError) -> Self {
        Self::StateError(StateError::TokenOrderNotOwnedError(err))
    }
}
