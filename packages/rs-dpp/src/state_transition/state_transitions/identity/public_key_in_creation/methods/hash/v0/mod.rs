use crate::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
use crate::identity::IdentityPublicKey;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::ProtocolError;

impl IdentityPublicKeyInCreation {
    /// Get the original public key hash
    #[inline(always)]
    pub(super) fn hash_v0(&self) -> Result<[u8; 20], ProtocolError> {
        Into::<IdentityPublicKey>::into(self.clone()).hash()
    }
}
