mod v0;

use crate::identity::IdentityPublicKey;

use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::{BlsModule, ProtocolError};
use platform_version::version::PlatformVersion;

impl IdentityPublicKeyInCreation {
    pub fn from_public_key_signed_with_private_key(
        public_key: IdentityPublicKey,
        state_transition_bytes: &[u8],
        private_key: &[u8],
        bls: &impl BlsModule,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_method_versions
            .public_key_in_creation_methods
            .from_public_key_signed_with_private_key
        {
            0 => Self::from_public_key_signed_with_private_key_v0(
                public_key,
                state_transition_bytes,
                private_key,
                bls,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityPublicKeyInCreation::from_public_key_signed_with_private_key"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
