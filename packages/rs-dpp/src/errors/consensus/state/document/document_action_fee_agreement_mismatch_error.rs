use crate::balances::credits::Credits;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::action_fees::agreement::DocumentActionFeeAgreement;
use crate::data_contract::document_type::action_fees::{ActionFeePricing, DocumentActionFee};
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// The transition's action fee agreement does not name what the document type declares for
/// the action: an amount or the pricing differs. The signer reads the contract again and
/// agrees to what it declares now.
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
    "Document {} of type {} charges an action fee of {} credits to the owner and {} credits to the moderators ({} pricing), but the transition agreed to {} and {} credits ({} pricing)",
    action,
    document_type_name,
    declared_owner,
    declared_moderators,
    declared_pricing,
    agreed_owner,
    agreed_moderators,
    agreed_pricing
)]
#[platform_serialize(unversioned)]
pub struct DocumentActionFeeAgreementMismatchError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type_name: String,
    action: String,
    declared_owner: Credits,
    declared_moderators: Credits,
    declared_pricing: ActionFeePricing,
    agreed_owner: Credits,
    agreed_moderators: Credits,
    agreed_pricing: ActionFeePricing,
}

impl DocumentActionFeeAgreementMismatchError {
    pub fn new(
        document_type_name: String,
        action: String,
        declared_pricing: ActionFeePricing,
        declared_fee: DocumentActionFee,
        agreement: &DocumentActionFeeAgreement,
    ) -> Self {
        Self {
            document_type_name,
            action,
            declared_owner: declared_fee.owner,
            declared_moderators: declared_fee.moderators,
            declared_pricing,
            agreed_owner: agreement.owner(),
            agreed_moderators: agreement.moderators(),
            agreed_pricing: agreement.pricing(),
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

    pub fn agreed_owner(&self) -> Credits {
        self.agreed_owner
    }

    pub fn agreed_moderators(&self) -> Credits {
        self.agreed_moderators
    }

    pub fn agreed_pricing(&self) -> ActionFeePricing {
        self.agreed_pricing
    }
}

impl From<DocumentActionFeeAgreementMismatchError> for ConsensusError {
    fn from(err: DocumentActionFeeAgreementMismatchError) -> Self {
        Self::StateError(StateError::DocumentActionFeeAgreementMismatchError(err))
    }
}
