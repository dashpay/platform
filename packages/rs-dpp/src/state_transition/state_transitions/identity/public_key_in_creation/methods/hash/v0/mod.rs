use crate::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
use crate::identity::identity_public_key::IdentityPublicKey;
use crate::state_transition::state_transitions::identity::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::ProtocolError;
use std::convert::TryInto;

impl IdentityPublicKeyInCreation {
    /// Get the original public key hash
    pub(super) fn hash_v0(&self) -> Result<[u8; 20], ProtocolError> {
        Into::<IdentityPublicKey>::into(self.clone())
            .hash()?
            .try_into()
            .map_err(|_| {
                ProtocolError::CorruptedCodeExecution(
                    "hash should always output 20 bytes".to_string(),
                )
            })
    }
}
