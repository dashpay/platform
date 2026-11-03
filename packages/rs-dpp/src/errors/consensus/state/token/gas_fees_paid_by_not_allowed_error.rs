use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::tokens::gas_fees_paid_by::GasFeesPaidBy;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// The transition asks the contract owner to pay the gas of a document action whose token cost
/// does not offer that (see [`GasFeesPaidBy::resolve`]). An action without a token cost offers
/// nothing.
#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error(
    "Document {} of type {} asks for gas fees paid by {}, but the document type only offers {}",
    action,
    document_type_name,
    requested,
    offered
)]
#[platform_serialize(unversioned)]
pub struct GasFeesPaidByNotAllowedError {
    document_type_name: String,
    action: String,
    requested: GasFeesPaidBy,
    offered: GasFeesPaidBy,
}

impl GasFeesPaidByNotAllowedError {
    pub fn new(
        document_type_name: String,
        action: String,
        requested: GasFeesPaidBy,
        offered: GasFeesPaidBy,
    ) -> Self {
        Self {
            document_type_name,
            action,
            requested,
            offered,
        }
    }

    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }

    pub fn action(&self) -> &str {
        &self.action
    }

    pub fn requested(&self) -> GasFeesPaidBy {
        self.requested
    }

    pub fn offered(&self) -> GasFeesPaidBy {
        self.offered
    }
}

impl From<GasFeesPaidByNotAllowedError> for ConsensusError {
    fn from(err: GasFeesPaidByNotAllowedError) -> Self {
        Self::StateError(StateError::GasFeesPaidByNotAllowedError(err))
    }
}
