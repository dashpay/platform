use crate::balances::credits::TokenAmount;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// No amount between the fill's minimum and maximum can be taken from the order: less is open than
/// the fill's minimum, or the amount would be under the order's minimum fill without taking
/// everything open, or would leave less than the minimum fill open.
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
#[error("A fill of token order {order_id} between {min_amount} and {max_amount} cannot be taken: {remaining} is open, and a fill takes at least {min_fill} and leaves none or at least {min_fill}")]
#[platform_serialize(unversioned)]
pub struct TokenOrderFillAmountError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    order_id: Identifier,
    min_amount: TokenAmount,
    max_amount: TokenAmount,
    remaining: TokenAmount,
    min_fill: TokenAmount,
}

impl TokenOrderFillAmountError {
    pub fn new(
        order_id: Identifier,
        min_amount: TokenAmount,
        max_amount: TokenAmount,
        remaining: TokenAmount,
        min_fill: TokenAmount,
    ) -> Self {
        Self {
            order_id,
            min_amount,
            max_amount,
            remaining,
            min_fill,
        }
    }

    pub fn order_id(&self) -> Identifier {
        self.order_id
    }

    pub fn min_amount(&self) -> TokenAmount {
        self.min_amount
    }

    pub fn max_amount(&self) -> TokenAmount {
        self.max_amount
    }

    pub fn remaining(&self) -> TokenAmount {
        self.remaining
    }

    pub fn min_fill(&self) -> TokenAmount {
        self.min_fill
    }
}

impl From<TokenOrderFillAmountError> for ConsensusError {
    fn from(err: TokenOrderFillAmountError) -> Self {
        Self::StateError(StateError::TokenOrderFillAmountError(err))
    }
}
