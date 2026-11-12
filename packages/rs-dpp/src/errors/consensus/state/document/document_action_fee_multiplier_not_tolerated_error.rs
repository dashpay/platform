use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::action_fees::agreement::AgreedFeeMultiplier;
use crate::errors::ProtocolError;
use crate::prelude::FeeMultiplier;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// The fee multiplier of the epoch the action executes in is further above the one the
/// transition's signer knew than they accepted, so the action fee would cost more than they
/// agreed to.
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
    "Document {} of type {} agreed to an action fee priced with a fee multiplier of {} permille and at most {}% more, but the fee multiplier is {} permille",
    action,
    document_type_name,
    known_fee_multiplier_permille,
    increase_tolerance_percent,
    current_fee_multiplier_permille
)]
#[platform_serialize(unversioned)]
pub struct DocumentActionFeeMultiplierNotToleratedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type_name: String,
    action: String,
    known_fee_multiplier_permille: FeeMultiplier,
    increase_tolerance_percent: u16,
    current_fee_multiplier_permille: FeeMultiplier,
}

impl DocumentActionFeeMultiplierNotToleratedError {
    pub fn new(
        document_type_name: String,
        action: String,
        agreed: AgreedFeeMultiplier,
        current_fee_multiplier_permille: FeeMultiplier,
    ) -> Self {
        Self {
            document_type_name,
            action,
            known_fee_multiplier_permille: agreed.known_permille,
            increase_tolerance_percent: agreed.increase_tolerance_percent,
            current_fee_multiplier_permille,
        }
    }

    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }

    pub fn action(&self) -> &str {
        &self.action
    }

    pub fn known_fee_multiplier_permille(&self) -> FeeMultiplier {
        self.known_fee_multiplier_permille
    }

    pub fn increase_tolerance_percent(&self) -> u16 {
        self.increase_tolerance_percent
    }

    pub fn current_fee_multiplier_permille(&self) -> FeeMultiplier {
        self.current_fee_multiplier_permille
    }
}

impl From<DocumentActionFeeMultiplierNotToleratedError> for ConsensusError {
    fn from(err: DocumentActionFeeMultiplierNotToleratedError) -> Self {
        Self::StateError(StateError::DocumentActionFeeMultiplierNotToleratedError(
            err,
        ))
    }
}
