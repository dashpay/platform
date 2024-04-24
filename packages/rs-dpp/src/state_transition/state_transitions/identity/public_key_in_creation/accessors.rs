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
    fn set_id(&mut self, id: KeyID);
    fn set_type(&mut self, key_type: KeyType);
    fn set_data(&mut self, data: BinaryData);
    fn set_purpose(&mut self, purpose: Purpose);

    fn set_security_level(&mut self, security_level: SecurityLevel);

    fn set_contract_bounds(&mut self, contract_bounds: Option<ContractBounds>);

    fn set_read_only(&mut self, read_only: bool);
}

impl IdentityPublicKeyInCreationV0Setters for IdentityPublicKeyInCreation {
    fn set_signature(&mut self, signature: BinaryData) {
        match self {
            IdentityPublicKeyInCreation::V0(v0) => v0.signature = signature,
        }
    }

    fn set_id(&mut self, id: KeyID) {
        match self {
            IdentityPublicKeyInCreation::V0(v0) => v0.id = id,
        }
    }

    fn set_type(&mut self, key_type: KeyType) {
        match self {
            IdentityPublicKeyInCreation::V0(v0) => v0.key_type = key_type,
        }
    }

    fn set_data(&mut self, data: BinaryData) {
        match self {
            IdentityPublicKeyInCreation::V0(v0) => v0.data = data,
        }
    }

    fn set_purpose(&mut self, purpose: Purpose) {
        match self {
            IdentityPublicKeyInCreation::V0(v0) => v0.purpose = purpose,
        }
    }

    fn set_security_level(&mut self, security_level: SecurityLevel) {
        match self {
            IdentityPublicKeyInCreation::V0(v0) => v0.security_level = security_level,
        }
    }

    fn set_contract_bounds(&mut self, contract_bounds: Option<ContractBounds>) {
        match self {
            IdentityPublicKeyInCreation::V0(v0) => v0.contract_bounds = contract_bounds,
        }
    }

    fn set_read_only(&mut self, read_only: bool) {
        match self {
            IdentityPublicKeyInCreation::V0(v0) => v0.read_only = read_only,
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
