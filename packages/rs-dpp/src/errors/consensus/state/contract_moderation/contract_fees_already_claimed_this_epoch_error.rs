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
    "The {} fee pot of contract {} was already claimed in epoch {}; a pot is claimed at most once per epoch",
    pot,
    contract_id,
    epoch_index
)]
#[platform_serialize(unversioned)]
pub struct ContractFeesAlreadyClaimedThisEpochError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    pot: ContractFeePot,
    epoch_index: u16,
}

impl ContractFeesAlreadyClaimedThisEpochError {
    pub fn new(contract_id: Identifier, pot: ContractFeePot, epoch_index: u16) -> Self {
        Self {
            contract_id,
            pot,
            epoch_index,
        }
    }

    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    pub fn pot(&self) -> ContractFeePot {
        self.pot
    }

    pub fn epoch_index(&self) -> u16 {
        self.epoch_index
    }
}

impl From<ContractFeesAlreadyClaimedThisEpochError> for ConsensusError {
    fn from(err: ContractFeesAlreadyClaimedThisEpochError) -> Self {
        Self::StateError(StateError::ContractFeesAlreadyClaimedThisEpochError(err))
    }
}
