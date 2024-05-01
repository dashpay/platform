use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

pub mod v0;

impl IdentityPublicKeyInCreation {
    pub fn validate_identity_public_keys_structure(
        identity_public_keys_with_witness: &[IdentityPublicKeyInCreation],
        in_create_identity: bool,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_method_versions
            .public_key_in_creation_methods
            .validate_identity_public_keys_structure
        {
            0 => Self::validate_identity_public_keys_structure_v0(
                identity_public_keys_with_witness,
                in_create_identity,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityPublicKeyInCreation::validate_identity_public_keys_structure"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
