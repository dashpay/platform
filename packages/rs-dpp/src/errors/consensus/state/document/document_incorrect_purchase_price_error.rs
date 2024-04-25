use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::fee::Credits;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("{document_id} document can not be purchased for {trying_to_purchase_at_price}, it's sale price is {actual_price} (in credits)")]
#[platform_serialize(unversioned)]
pub struct DocumentIncorrectPurchasePriceError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_id: Identifier,

    trying_to_purchase_at_price: Credits,

    actual_price: Credits,
}

impl DocumentIncorrectPurchasePriceError {
    pub fn new(
        document_id: Identifier,
        trying_to_purchase_at_price: Credits,
        actual_price: Credits,
    ) -> Self {
        Self {
            document_id,
            trying_to_purchase_at_price,
            actual_price,
        }
    }

    pub fn document_id(&self) -> &Identifier {
        &self.document_id
    }

    pub fn trying_to_purchase_at_price(&self) -> Credits {
        self.trying_to_purchase_at_price
    }

    pub fn actual_price(&self) -> Credits {
        self.actual_price
    }
}

impl From<DocumentIncorrectPurchasePriceError> for ConsensusError {
    fn from(err: DocumentIncorrectPurchasePriceError) -> Self {
        Self::StateError(StateError::DocumentIncorrectPurchasePriceError(err))
    }
}
