use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::validation::SimpleConsensusValidationResult;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use dpp::version::PlatformVersion;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::common::validate_unique_identity_public_key_hashes_in_state::v0::validate_unique_identity_public_key_hashes_not_in_state_v0;
use crate::execution::validation::state_transition::common::validate_unique_identity_public_key_hashes_in_state::v1::validate_unique_identity_public_key_hashes_not_in_state_v1;

pub mod v0;
pub mod v1;

pub(crate) fn validate_unique_identity_public_key_hashes_not_in_state(
    identity_public_keys_with_witness: &[IdentityPublicKeyInCreation],
    drive: &Drive,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .common_validation_methods
        .validate_unique_identity_public_key_hashes_in_state
    {
        0 => validate_unique_identity_public_key_hashes_not_in_state_v0(
            identity_public_keys_with_witness,
            drive,
            execution_context,
            transaction,
            platform_version,
        ),
        1 => validate_unique_identity_public_key_hashes_not_in_state_v1(
            identity_public_keys_with_witness,
            drive,
            execution_context,
            transaction,
            platform_version,
        ),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "validate_unique_identity_public_key_hashes_in_state".to_string(),
            known_versions: vec![0, 1],
            received: version,
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::execution::ExecutionError;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::version::DefaultForPlatformVersion;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_return_unknown_version_error() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .common_validation_methods
            .validate_unique_identity_public_key_hashes_in_state = 99;

        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut execution_context = StateTransitionExecutionContext::default_for_platform_version(
            PlatformVersion::latest(),
        )
        .expect("should create execution context");

        let result = validate_unique_identity_public_key_hashes_not_in_state(
            &[],
            &platform.drive,
            &mut execution_context,
            None,
            &platform_version,
        );

        match result {
            Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method,
                known_versions,
                received,
            })) => {
                assert_eq!(
                    method,
                    "validate_unique_identity_public_key_hashes_in_state"
                );
                assert_eq!(known_versions, vec![0, 1]);
                assert_eq!(received, 99);
            }
            other => panic!("expected UnknownVersionMismatch error, got {:?}", other),
        }
    }
}
