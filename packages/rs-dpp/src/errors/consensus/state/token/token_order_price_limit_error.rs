use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::fee::Credits;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// The credits a fill moves are past the limit the filler set: more than its maximum for a
/// purchase, less than its minimum for a sale.
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
#[error(
    "The fill of token order {order_id} moves {total} credits, past the filler's limit of {limit}"
)]
#[platform_serialize(unversioned)]
pub struct TokenOrderPriceLimitError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    order_id: Identifier,
    total: Credits,
    limit: Credits,
}

impl TokenOrderPriceLimitError {
    pub fn new(order_id: Identifier, total: Credits, limit: Credits) -> Self {
        Self {
            order_id,
            total,
            limit,
        }
    }

    pub fn order_id(&self) -> Identifier {
        self.order_id
    }

    pub fn total(&self) -> Credits {
        self.total
    }

    pub fn limit(&self) -> Credits {
        self.limit
    }
}

impl From<TokenOrderPriceLimitError> for ConsensusError {
    fn from(err: TokenOrderPriceLimitError) -> Self {
        Self::StateError(StateError::TokenOrderPriceLimitError(err))
    }
}
