pub mod duplicated_key_ids_witness;
mod duplicated_keys_witness;
#[cfg(feature = "state-transition-signing")]
pub mod from_public_key_signed_external;
#[cfg(feature = "state-transition-signing")]
pub mod from_public_key_signed_with_private_key;
pub mod hash;
mod v0;

use crate::identity::signer::Signer;
use crate::identity::IdentityPublicKey;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::{BlsModule, ProtocolError};
pub use v0::IdentityPublicKeyInCreationMethodsV0;

impl IdentityPublicKeyInCreationMethodsV0 for IdentityPublicKeyInCreation {
    fn into_identity_public_key(self) -> IdentityPublicKey {
        match self {
            IdentityPublicKeyInCreation::V0(v0) => v0.into_identity_public_key(),
        }
    }
}
