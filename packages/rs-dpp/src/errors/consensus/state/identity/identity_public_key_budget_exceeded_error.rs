use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::fee::Credits;
use crate::identity::KeyID;
use crate::prelude::Identifier;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
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
#[error("Identity {identity_id} public key {public_key_id} has {remaining_budget} credits of budget left, the state transition requires {required_budget}")]
#[platform_serialize(unversioned)]
pub struct IdentityPublicKeyBudgetExceededError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    identity_id: Identifier,
    public_key_id: KeyID,
    remaining_budget: Credits,
    required_budget: Credits,
}

impl IdentityPublicKeyBudgetExceededError {
    pub fn new(
        identity_id: Identifier,
        public_key_id: KeyID,
        remaining_budget: Credits,
        required_budget: Credits,
    ) -> Self {
        Self {
            identity_id,
            public_key_id,
            remaining_budget,
            required_budget,
        }
    }

    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }

    pub fn public_key_id(&self) -> KeyID {
        self.public_key_id
    }

    pub fn remaining_budget(&self) -> Credits {
        self.remaining_budget
    }

    pub fn required_budget(&self) -> Credits {
        self.required_budget
    }
}

impl From<IdentityPublicKeyBudgetExceededError> for ConsensusError {
    fn from(err: IdentityPublicKeyBudgetExceededError) -> Self {
        Self::StateError(StateError::IdentityPublicKeyBudgetExceededError(err))
    }
}
