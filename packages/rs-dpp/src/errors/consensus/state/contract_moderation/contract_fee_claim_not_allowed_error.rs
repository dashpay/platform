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
    "Identity {} is not a recipient of the {} fee pot of contract {} and can not claim it",
    identity_id,
    pot,
    contract_id
)]
#[platform_serialize(unversioned)]
pub struct ContractFeeClaimNotAllowedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    pot: ContractFeePot,
    identity_id: Identifier,
}

impl ContractFeeClaimNotAllowedError {
    pub fn new(contract_id: Identifier, pot: ContractFeePot, identity_id: Identifier) -> Self {
        Self {
            contract_id,
            pot,
            identity_id,
        }
    }

    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    pub fn pot(&self) -> ContractFeePot {
        self.pot
    }

    pub fn identity_id(&self) -> Identifier {
        self.identity_id
    }
}

impl From<ContractFeeClaimNotAllowedError> for ConsensusError {
    fn from(err: ContractFeeClaimNotAllowedError) -> Self {
        Self::StateError(StateError::ContractFeeClaimNotAllowedError(err))
    }
}
