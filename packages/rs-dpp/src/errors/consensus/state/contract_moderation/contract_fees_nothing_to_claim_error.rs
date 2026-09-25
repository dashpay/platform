use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::action_fees::ContractFeePot;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

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
    "The {} fee pot of contract {} holds nothing that can be paid out",
    pot,
    contract_id
)]
#[platform_serialize(unversioned)]
pub struct ContractFeesNothingToClaimError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    pot: ContractFeePot,
}

impl ContractFeesNothingToClaimError {
    pub fn new(contract_id: Identifier, pot: ContractFeePot) -> Self {
        Self { contract_id, pot }
    }

    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    pub fn pot(&self) -> ContractFeePot {
        self.pot
    }
}

impl From<ContractFeesNothingToClaimError> for ConsensusError {
    fn from(err: ContractFeesNothingToClaimError) -> Self {
        Self::StateError(StateError::ContractFeesNothingToClaimError(err))
    }
}
