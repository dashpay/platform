use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::prelude::Identifier;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// The transition insists that the contract owner pays its gas, and the contract owner's balance
/// does not cover the fee. Nobody is charged: the document owner refused to pay by asking for
/// `ContractOwner` rather than `PreferContractOwner`.
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
    "The contract owner {sponsor_id} sponsoring the gas has balance {balance}, but {required_balance} is required"
)]
#[platform_serialize(unversioned)]
pub struct GasSponsorInsufficientBalanceError {
    sponsor_id: Identifier,
    balance: u64,
    required_balance: u64,
}

impl GasSponsorInsufficientBalanceError {
    pub fn new(sponsor_id: Identifier, balance: u64, required_balance: u64) -> Self {
        Self {
            sponsor_id,
            balance,
            required_balance,
        }
    }

    pub fn sponsor_id(&self) -> &Identifier {
        &self.sponsor_id
    }

    pub fn balance(&self) -> u64 {
        self.balance
    }

    pub fn required_balance(&self) -> u64 {
        self.required_balance
    }
}

impl From<GasSponsorInsufficientBalanceError> for ConsensusError {
    fn from(err: GasSponsorInsufficientBalanceError) -> Self {
        Self::StateError(StateError::GasSponsorInsufficientBalanceError(err))
    }
}
