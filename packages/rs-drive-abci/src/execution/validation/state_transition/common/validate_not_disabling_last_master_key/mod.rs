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
