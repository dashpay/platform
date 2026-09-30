use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A batch is paid for as a whole, so a contract owner sponsors either every transition in it or
/// none. The batch mixes transitions the document owner pays for with sponsored ones, or asks two
/// different contract owners to pay. `None` stands for the document owner.
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
    "The gas of a batch is paid by one identity: it cannot be paid by {} for one transition and by {} for another",
    payer_description(expected_payer),
    payer_description(found_payer)
)]
#[platform_serialize(unversioned)]
pub struct InconsistentGasFeesPaidByInBatchError {
    expected_payer: Option<Identifier>,
    found_payer: Option<Identifier>,
}

fn payer_description(payer: &Option<Identifier>) -> String {
    match payer {
        Some(contract_owner_id) => format!("the contract owner {}", contract_owner_id),
        None => "the document owner".to_string(),
    }
}

impl InconsistentGasFeesPaidByInBatchError {
    pub fn new(expected_payer: Option<Identifier>, found_payer: Option<Identifier>) -> Self {
        Self {
            expected_payer,
            found_payer,
        }
    }

    /// The contract owner the first sponsored transition named, or `None` for the document owner
    pub fn expected_payer(&self) -> Option<Identifier> {
        self.expected_payer
    }

    /// The payer a later transition resolved to, or `None` for the document owner
    pub fn found_payer(&self) -> Option<Identifier> {
        self.found_payer
    }
}

impl From<InconsistentGasFeesPaidByInBatchError> for ConsensusError {
    fn from(err: InconsistentGasFeesPaidByInBatchError) -> Self {
        Self::StateError(StateError::InconsistentGasFeesPaidByInBatchError(err))
    }
}
