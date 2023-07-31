mod v0;

use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::identity::IdentityPublicKey;
use crate::serialization::PlatformMessageSignable;
use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Setters;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::{BlsModule, ProtocolError};
use platform_version::version::PlatformVersion;

impl IdentityPublicKeyInCreation {
    pub fn hash(&self, platform_version: &PlatformVersion) -> Result<[u8; 20], ProtocolError> {
        match platform_version
            .dpp
            .state_transition_method_versions
            .public_key_in_creation_methods
            .hash
        {
            0 => self.hash_v0(),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityPublicKeyInCreation::hash".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    pub fn hash_as_vec(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_method_versions
            .public_key_in_creation_methods
            .hash
        {
            0 => self.hash_v0().map(|hash| hash.to_vec()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityPublicKeyInCreation::hash".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
