use dpp::identity::IdentityPublicKey;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::execution::validation::state_transition::common::validate_not_disabling_last_master_key::v0::validate_master_key_uniqueness_v0;

pub mod v0;

pub(crate) fn validate_master_key_uniqueness(
    public_keys_being_added: &[IdentityPublicKeyInCreation],
    public_keys_to_disable: &[IdentityPublicKey],
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .common_validation_methods
        .validate_master_key_uniqueness
    {
        0 => validate_master_key_uniqueness_v0(public_keys_being_added, public_keys_to_disable),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "validate_not_disabling_last_master_key".to_string(),
            known_versions: vec![0],
            received: version,
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::execution::ExecutionError;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_pass_with_valid_version() {
        let platform_version = PlatformVersion::latest();
        let result =
            validate_master_key_uniqueness(&[], &[], platform_version).expect("should succeed");
        assert!(
            result.is_valid(),
            "should be valid with empty keys and valid version"
        );
    }

    #[test]
    fn should_return_unknown_version_error() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .common_validation_methods
            .validate_master_key_uniqueness = 99;
        let result = validate_master_key_uniqueness(&[], &[], &platform_version);
        match result {
            Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method,
                known_versions,
                received,
            })) => {
                assert_eq!(method, "validate_not_disabling_last_master_key");
                assert_eq!(known_versions, vec![0]);
                assert_eq!(received, 99);
            }
            other => panic!("expected UnknownVersionMismatch error, got {:?}", other),
        }
    }
}
