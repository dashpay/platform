mod v0;

use crate::identity::signer::Signer;
use crate::identity::IdentityPublicKey;

use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl IdentityPublicKeyInCreation {
    pub fn from_public_key_signed_external<S: Signer>(
        public_key: IdentityPublicKey,
        state_transition_bytes: &[u8],
        signer: &S,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_method_versions
            .public_key_in_creation_methods
            .from_public_key_signed_external
        {
            0 => {
                Self::from_public_key_signed_external_v0(public_key, state_transition_bytes, signer)
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityPublicKeyInCreation::from_public_key_signed_external".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
