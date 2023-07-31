use platform_version::version::PlatformVersion;
use crate::identity::KeyID;
use crate::ProtocolError;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;

mod v0;

impl IdentityPublicKeyInCreation {
    pub fn duplicated_key_ids_witness(public_keys: &[IdentityPublicKeyInCreation], platform_version: &PlatformVersion) -> Result<Vec<KeyID>, ProtocolError> {
        match platform_version.dpp.state_transition_method_versions.public_key_in_creation_methods.duplicated_key_ids_witness {
            0 => Ok(Self::duplicated_key_ids_witness_v0(public_keys)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityPublicKeyInCreation::duplicated_key_ids_witness".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}