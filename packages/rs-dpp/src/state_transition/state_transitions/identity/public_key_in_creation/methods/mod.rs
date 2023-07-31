mod v0;
#[cfg(feature = "state-transition-signing")]
pub mod from_public_key_signed_with_private_key;
#[cfg(feature = "state-transition-signing")]
pub mod from_public_key_signed_external;
pub mod duplicated_key_ids_witness;
pub mod hash;
mod duplicated_keys_witness;

pub use v0::IdentityPublicKeyInCreationMethodsV0;
use crate::{BlsModule, ProtocolError};
use crate::identity::IdentityPublicKey;
use crate::identity::signer::Signer;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;

impl IdentityPublicKeyInCreationMethodsV0 for IdentityPublicKeyInCreation {
    fn into_identity_public_key(self) -> IdentityPublicKey {
        match self { IdentityPublicKeyInCreation::V0(v0) => v0.into_identity_public_key() }
    }
}