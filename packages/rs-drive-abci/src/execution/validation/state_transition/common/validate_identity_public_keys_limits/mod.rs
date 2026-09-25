use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::common::validate_identity_public_keys_limits::v0::validate_identity_public_keys_limits_v0;
use dpp::block::block_info::BlockInfo;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub mod v0;

/// Validates the usage limits of public keys being added against the block they are added in:
/// a key must not already be expired when it is registered.
///
/// Which keys may carry limits at all is a structure rule and lives in
/// `IdentityPublicKeyInCreation::validate_identity_public_keys_structure`; this is the part that
/// needs the block time.
///
/// # Parameters
/// - `identity_public_keys_with_witness`: The public keys being added.
/// - `block_info`: The block the keys are added in.
/// - `platform_version`: The platform version selecting the implementation.
///
/// # Returns
/// - One `IdentityPublicKeyAlreadyExpiredError` per key whose expiry is not after the block time.
pub(crate) fn validate_identity_public_keys_limits(
    identity_public_keys_with_witness: &[IdentityPublicKeyInCreation],
    block_info: &BlockInfo,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .common_validation_methods
        .validate_identity_public_keys_limits
    {
        Some(0) => Ok(validate_identity_public_keys_limits_v0(
            identity_public_keys_with_witness,
            block_info,
        )),
        Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "validate_identity_public_keys_limits".to_string(),
            known_versions: vec![0],
            received: version,
        })),
        None => Err(Error::Execution(ExecutionError::VersionNotActive {
            method: "validate_identity_public_keys_limits".to_string(),
            known_versions: vec![0],
        })),
    }
}
