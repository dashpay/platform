use crate::identity::contract_bounds::ContractBounds;
use crate::identity::{KeyID, KeyType, Purpose, SecurityLevel};
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use platform_value::BinaryData;

/// Trait providing getters for `IdentityPublicKeyInCreationV0`.
pub trait IdentityPublicKeyInCreationV0Getters {
    /// Returns the `KeyID`.
    fn id(&self) -> KeyID;

    /// Returns the cryptographic `KeyType`.
    fn key_type(&self) -> KeyType;

    /// Returns the `Purpose` of the key.
    fn purpose(&self) -> Purpose;

    /// Returns the `SecurityLevel`.
    fn security_level(&self) -> SecurityLevel;

    /// Indicates if the key is read-only.
    fn read_only(&self) -> bool;

    /// Returns the binary `data` of the key.
    fn data(&self) -> &BinaryData;

    /// Returns the key's `signature`.
    fn signature(&self) -> &BinaryData;

    /// Contract bounds
    fn contract_bounds(&self) -> Option<&ContractBounds>;
}

/// Trait providing getters for `IdentityPublicKeyInCreationV0`.
pub trait IdentityPublicKeyInCreationV0Setters {
    fn set_signature(&mut self, signature: BinaryData);
}

impl IdentityPublicKeyInCreationV0Setters for IdentityPublicKeyInCreation {
    fn set_signature(&mut self, signature: BinaryData) {
        match self {
            IdentityPublicKeyInCreation::V0(v0) => v0.signature = signature,
        }
    }
}

// Implement the getter trait for the struct
impl IdentityPublicKeyInCreationV0Getters for IdentityPublicKeyInCreation {
    fn id(&self) -> KeyID {
        match self {
            IdentityPublicKeyInCreation::V0(v0) => v0.id,
        }
    }

    fn key_type(&self) -> KeyType {
        match self {
            IdentityPublicKeyInCreation::V0(v0) => v0.key_type,
        }
    }

    fn purpose(&self) -> Purpose {
        match self {
            IdentityPublicKeyInCreation::V0(v0) => v0.purpose,
        }
    }

    fn security_level(&self) -> SecurityLevel {
        match self {
            IdentityPublicKeyInCreation::V0(v0) => v0.security_level,
        }
    }

    fn read_only(&self) -> bool {
        match self {
            IdentityPublicKeyInCreation::V0(v0) => v0.read_only,
        }
    }

    fn data(&self) -> &BinaryData {
        match self {
            IdentityPublicKeyInCreation::V0(v0) => &v0.data,
        }
    }

    fn signature(&self) -> &BinaryData {
        match self {
            IdentityPublicKeyInCreation::V0(v0) => &v0.signature,
        }
    }

    fn contract_bounds(&self) -> Option<&ContractBounds> {
        match self {
            IdentityPublicKeyInCreation::V0(v0) => v0.contract_bounds.as_ref(),
        }
    }
}
