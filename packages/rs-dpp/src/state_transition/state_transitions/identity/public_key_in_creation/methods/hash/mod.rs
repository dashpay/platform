mod v0;

use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl IdentityPublicKeyInCreation {
    pub fn hash(&self) -> Result<[u8; 20], ProtocolError> {
        match self {
            IdentityPublicKeyInCreation::V0(v0) => self.hash_v0(),
        }
    }

    pub fn hash_as_vec(
        &self,
    ) -> Result<Vec<u8>, ProtocolError> {
        match self {
            IdentityPublicKeyInCreation::V0(v0) => self.hash_v0().map(|hash| hash.to_vec()),
        }
    }
}
