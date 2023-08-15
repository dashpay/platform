use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;

use crate::identity::{IdentityPublicKey, SecurityLevel};

use crate::state_transition::public_key_in_creation::accessors::{
    IdentityPublicKeyInCreationV0Getters, IdentityPublicKeyInCreationV0Setters,
};

pub trait IdentityPublicKeyInCreationMethodsV0:
    IdentityPublicKeyInCreationV0Getters + IdentityPublicKeyInCreationV0Setters
{
    fn into_identity_public_key(self) -> IdentityPublicKey;

    /// Checks if public key security level is MASTER
    fn is_master(&self) -> bool {
        self.security_level() == SecurityLevel::MASTER
    }
}
