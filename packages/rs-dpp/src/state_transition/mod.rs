mod state_transition_types;
pub use state_transition_types::*;

mod abstract_state_transition;
pub use abstract_state_transition::{StateTransitionConvert, StateTransitionLike};

mod signer;

mod calculate_state_transition_fee;

use crate::mocks;
/// Methods dispatcher for Static Transition
macro_rules! call_method {
    ($state_transition:expr, $method:ident, $args:tt ) => {
        match $state_transition {
            StateTransition::DataContractCreate(st) => st.$method($args),
            StateTransition::DataContractUpdate(st) => st.$method($args),
            StateTransition::DocumentsBatch(st) => st.$method($args),
            StateTransition::IdentityCreate(st) => st.$method($args),
            StateTransition::IdentityTopUp(st) => st.$method($args),
        }
    };
    ($state_transition:expr, $method:ident ) => {
        match $state_transition {
            StateTransition::DataContractCreate(st) => st.$method(),
            StateTransition::DataContractUpdate(st) => st.$method(),
            StateTransition::DocumentsBatch(st) => st.$method(),
            StateTransition::IdentityCreate(st) => st.$method(),
            StateTransition::IdentityTopUp(st) => st.$method(),
        }
    };
}

#[derive(Debug, Clone)]
pub enum StateTransition {
    DataContractCreate(mocks::DataContractCreateTransition),
    DataContractUpdate(mocks::DataContractUpdateTransition),
    DocumentsBatch(mocks::DocumentsBatchTransition),
    IdentityCreate(mocks::IdentityCreateTransition),
    IdentityTopUp(mocks::IdentityTopUpTransition),
}

impl StateTransitionConvert for StateTransition {
    fn hash(&self, skip_signature: bool) -> Result<Vec<u8>, crate::ProtocolError> {
        call_method!(self, hash, skip_signature)
    }
    fn to_buffer(&self, _skip_signature: bool) -> Result<Vec<u8>, crate::ProtocolError> {
        call_method!(self, to_buffer, true)
    }

    fn to_json(&self) -> Result<serde_json::Value, crate::ProtocolError> {
        call_method!(self, to_json)
    }

    fn to_object(&self, skip_signature: bool) -> Result<serde_json::Value, crate::ProtocolError> {
        call_method!(self, to_object, skip_signature)
    }
}

impl StateTransitionLike for StateTransition {
    fn get_protocol_version(&self) -> u32 {
        call_method!(self, get_protocol_version)
    }
    /// returns the type of State Transition
    fn get_type(&self) -> StateTransitionType {
        call_method!(self, get_type)
    }
    /// returns the signature as a byte-array
    fn get_signature(&self) -> &Vec<u8> {
        call_method!(self, get_signature)
    }

    /// set a new signature
    fn set_signature(&mut self, signature: Vec<u8>) {
        call_method!(self, set_signature, signature)
    }

    fn calculate_fee(&self) -> Result<u64, crate::ProtocolError> {
        call_method!(self, calculate_fee)
    }
}

impl From<mocks::DataContractCreateTransition> for StateTransition {
    fn from(d: mocks::DataContractCreateTransition) -> Self {
        Self::DataContractCreate(d)
    }
}

impl From<mocks::DataContractUpdateTransition> for StateTransition {
    fn from(d: mocks::DataContractUpdateTransition) -> Self {
        Self::DataContractUpdate(d)
    }
}

impl From<mocks::DocumentsBatchTransition> for StateTransition {
    fn from(d: mocks::DocumentsBatchTransition) -> Self {
        Self::DocumentsBatch(d)
    }
}

impl From<mocks::IdentityCreateTransition> for StateTransition {
    fn from(d: mocks::IdentityCreateTransition) -> Self {
        Self::IdentityCreate(d)
    }
}

impl From<mocks::IdentityTopUpTransition> for StateTransition {
    fn from(d: mocks::IdentityTopUpTransition) -> Self {
        Self::IdentityTopUp(d)
    }
}
