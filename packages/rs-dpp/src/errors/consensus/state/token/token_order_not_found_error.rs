use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::tokens::token_order_side::TokenOrderSide;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// No open order of the side has this id for the token: it never existed, was filled entirely or
/// was cancelled.
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
#[error("Token order {order_id} ({side}) was not found")]
#[platform_serialize(unversioned)]
pub struct TokenOrderNotFoundError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    order_id: Identifier,
    side: TokenOrderSide,
}

impl TokenOrderNotFoundError {
    pub fn new(order_id: Identifier, side: TokenOrderSide) -> Self {
        Self { order_id, side }
    }

    pub fn order_id(&self) -> Identifier {
        self.order_id
    }

    pub fn side(&self) -> TokenOrderSide {
        self.side
    }
}

impl From<TokenOrderNotFoundError> for ConsensusError {
    fn from(err: TokenOrderNotFoundError) -> Self {
        Self::StateError(StateError::TokenOrderNotFoundError(err))
    }
}
