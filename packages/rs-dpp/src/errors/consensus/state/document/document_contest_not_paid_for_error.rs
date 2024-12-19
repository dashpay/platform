use crate::errors::consensus::state::state_error::StateError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::balances::credits::Credits;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Contest for document {document_id} was not paid for, needs payment of {expected_amount} Credits")]
#[platform_serialize(unversioned)]
#[ferment_macro::export]
pub struct DocumentContestNotPaidForError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub document_id: Identifier,

    pub expected_amount: Credits,

    pub paid_amount: Credits,
}

impl DocumentContestNotPaidForError {
    pub fn new(document_id: Identifier, expected_amount: Credits, paid_amount: Credits) -> Self {
        Self {
            document_id,
            expected_amount,
            paid_amount,
        }
    }

    pub fn document_id(&self) -> &Identifier {
        &self.document_id
    }

    pub fn expected_amount(&self) -> Credits {
        self.expected_amount
    }

    pub fn paid_amount(&self) -> Credits {
        self.paid_amount
    }
}

impl From<DocumentContestNotPaidForError> for ConsensusError {
    fn from(err: DocumentContestNotPaidForError) -> Self {
        Self::StateError(StateError::DocumentContestNotPaidForError(err))
    }
}
