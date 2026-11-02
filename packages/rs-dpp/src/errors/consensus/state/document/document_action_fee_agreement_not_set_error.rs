use crate::balances::credits::Credits;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::action_fees::{ActionFeePricing, DocumentActionFee};
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// The document type charges a fee for the action and the transition does not say what it
/// agrees to pay. The error names what the document type declares, which is what the
/// transition's action fee agreement must state.
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
    "Document {} of type {} charges an action fee of {} credits to the owner and {} credits to the moderators ({} pricing), and the transition carries no action fee agreement",
    action,
    document_type_name,
    declared_owner,
    declared_moderators,
    declared_pricing
)]
#[platform_serialize(unversioned)]
pub struct DocumentActionFeeAgreementNotSetError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type_name: String,
    action: String,
    declared_owner: Credits,
    declared_moderators: Credits,
    declared_pricing: ActionFeePricing,
}

impl DocumentActionFeeAgreementNotSetError {
    pub fn new(
        document_type_name: String,
        action: String,
        declared_pricing: ActionFeePricing,
        declared_fee: DocumentActionFee,
    ) -> Self {
        Self {
            document_type_name,
            action,
            declared_owner: declared_fee.owner,
            declared_moderators: declared_fee.moderators,
            declared_pricing,
        }
    }

    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }

    pub fn action(&self) -> &str {
        &self.action
    }

    pub fn declared_owner(&self) -> Credits {
        self.declared_owner
    }

    pub fn declared_moderators(&self) -> Credits {
        self.declared_moderators
    }

    pub fn declared_pricing(&self) -> ActionFeePricing {
        self.declared_pricing
    }
}

impl From<DocumentActionFeeAgreementNotSetError> for ConsensusError {
    fn from(err: DocumentActionFeeAgreementNotSetError) -> Self {
        Self::StateError(StateError::DocumentActionFeeAgreementNotSetError(err))
    }
}
