use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
use crate::identity::signer::Signer;
use crate::identity::{IdentityPublicKey, KeyID, KeyType, SecurityLevel};
use crate::serialization::PlatformMessageSignable;
use crate::state_transition::public_key_in_creation::accessors::{
    IdentityPublicKeyInCreationV0Getters, IdentityPublicKeyInCreationV0Setters,
};
use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::{BlsModule, ProtocolError};
use std::collections::HashMap;
use std::convert::TryInto;

pub trait IdentityPublicKeyInCreationMethodsV0:
    IdentityPublicKeyInCreationV0Getters + IdentityPublicKeyInCreationV0Setters
{
    fn into_identity_public_key(self) -> IdentityPublicKey;

    /// Checks if public key security level is MASTER
    fn is_master(&self) -> bool {
        self.security_level() == SecurityLevel::MASTER
    }
}
