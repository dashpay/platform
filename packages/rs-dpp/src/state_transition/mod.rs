mod state_transition_types;
pub use state_transition_types::*;

mod abstract_state_transition;
pub use abstract_state_transition::*;

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

#[derive(Debug)]
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
