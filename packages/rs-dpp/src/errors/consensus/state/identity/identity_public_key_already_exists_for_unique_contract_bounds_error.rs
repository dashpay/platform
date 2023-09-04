use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::{KeyID, Purpose};
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Identity Public Key with id ${new_key_id} for identity ${identity_id:?} conflicts for purpose ${purpose} with key ${old_key_id} in the contract bounds of ${contract_id:?}")]
#[platform_serialize(unversioned)]
pub struct IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    identity_id: Identifier,

    contract_id: Identifier,

    purpose: Purpose,

    new_key_id: KeyID,

    old_key_id: KeyID,
}

impl IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError {
    pub fn new(
        identity_id: Identifier,
        contract_id: Identifier,
        purpose: Purpose,
        new_key_id: KeyID,
        old_key_id: KeyID,
    ) -> Self {
        Self {
            identity_id,
            contract_id,
            purpose,
            new_key_id,
            old_key_id,
        }
    }

    pub fn identity_id(&self) -> Identifier {
        self.identity_id
    }

    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    pub fn purpose(&self) -> Purpose {
        self.purpose
    }

    pub fn new_key_id(&self) -> KeyID {
        self.new_key_id
    }

    pub fn old_key_id(&self) -> KeyID {
        self.old_key_id
    }
}

impl From<IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError> for ConsensusError {
    fn from(err: IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError) -> Self {
        Self::StateError(
            StateError::IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError(err),
        )
    }
}
