use serde::{Deserialize, Serialize};

pub use abstract_state_transition::{
    state_transition_helpers, StateTransitionConvert, StateTransitionLike,
};
pub use abstract_state_transition_identity_signed::StateTransitionIdentitySigned;
pub use state_transition_types::*;

use crate::data_contract::state_transition::{
    DataContractCreateTransition, DataContractUpdateTransition,
};
// TODO unify the import paths ::object::state_transition::*
use crate::document::DocumentsBatchTransition;
use crate::identity::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::identity::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use crate::identity::state_transition::identity_topup_transition::IdentityTopUpTransition;
use crate::identity::state_transition::identity_update_transition::identity_update_transition::IdentityUpdateTransition;

mod abstract_state_transition;
mod abstract_state_transition_identity_signed;
mod calculate_state_transition_fee;
mod state_transition_factory;
pub use state_transition_factory::*;

mod state_transition_types;
pub mod validation;

mod example;
macro_rules! call_method {
    ($state_transition:expr, $method:ident, $args:tt ) => {
        match $state_transition {
            StateTransition::DataContractCreate(st) => st.$method($args),
            StateTransition::DataContractUpdate(st) => st.$method($args),
            StateTransition::DocumentsBatch(st) => st.$method($args),
            StateTransition::IdentityCreate(st) => st.$method($args),
            StateTransition::IdentityTopUp(st) => st.$method($args),
            StateTransition::IdentityCreditWithdrawal(st) => st.$method($args),
            StateTransition::IdentityUpdate(st) => st.$method($args),
        }
    };
    ($state_transition:expr, $method:ident ) => {
        match $state_transition {
            StateTransition::DataContractCreate(st) => st.$method(),
            StateTransition::DataContractUpdate(st) => st.$method(),
            StateTransition::DocumentsBatch(st) => st.$method(),
            StateTransition::IdentityCreate(st) => st.$method(),
            StateTransition::IdentityTopUp(st) => st.$method(),
            StateTransition::IdentityCreditWithdrawal(st) => st.$method(),
            StateTransition::IdentityUpdate(st) => st.$method(),
        }
    };
}

macro_rules! call_static_method {
    ($state_transition:expr, $method:ident ) => {
        match $state_transition {
            StateTransition::DataContractCreate(_) => DataContractCreateTransition::$method(),
            StateTransition::DataContractUpdate(_) => DataContractUpdateTransition::$method(),
            StateTransition::DocumentsBatch(_) => DocumentsBatchTransition::$method(),
            StateTransition::IdentityCreate(_) => IdentityCreateTransition::$method(),
            StateTransition::IdentityTopUp(_) => IdentityTopUpTransition::$method(),
            StateTransition::IdentityCreditWithdrawal(_) => {
                IdentityCreditWithdrawalTransition::$method()
            }
            StateTransition::IdentityUpdate(_) => IdentityUpdateTransition::$method(),
        }
    };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateTransition {
    DataContractCreate(DataContractCreateTransition),
    DataContractUpdate(DataContractUpdateTransition),
    DocumentsBatch(DocumentsBatchTransition),
    IdentityCreate(IdentityCreateTransition),
    IdentityTopUp(IdentityTopUpTransition),
    IdentityCreditWithdrawal(IdentityCreditWithdrawalTransition),
    IdentityUpdate(IdentityUpdateTransition),
}

impl StateTransition {
    fn signature_property_paths(&self) -> Vec<&'static str> {
        call_static_method!(self, signature_property_paths)
    }

    fn identifiers_property_paths(&self) -> Vec<&'static str> {
        call_static_method!(self, identifiers_property_paths)
    }

    fn binary_property_paths(&self) -> Vec<&'static str> {
        call_static_method!(self, binary_property_paths)
    }
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

    fn signature_property_paths() -> Vec<&'static str> {
        panic!("Static call is not supported")
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        panic!("Static call is not supported")
    }

    fn binary_property_paths() -> Vec<&'static str> {
        panic!("Static call is not supported")
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

impl From<DataContractCreateTransition> for StateTransition {
    fn from(d: DataContractCreateTransition) -> Self {
        Self::DataContractCreate(d)
    }
}

impl From<DataContractUpdateTransition> for StateTransition {
    fn from(d: DataContractUpdateTransition) -> Self {
        Self::DataContractUpdate(d)
    }
}

impl From<DocumentsBatchTransition> for StateTransition {
    fn from(d: DocumentsBatchTransition) -> Self {
        Self::DocumentsBatch(d)
    }
}

impl From<IdentityCreditWithdrawalTransition> for StateTransition {
    fn from(d: IdentityCreditWithdrawalTransition) -> Self {
        Self::IdentityCreditWithdrawal(d)
    }
}

impl From<IdentityUpdateTransition> for StateTransition {
    fn from(d: IdentityUpdateTransition) -> Self {
        Self::IdentityUpdate(d)
    }
}
