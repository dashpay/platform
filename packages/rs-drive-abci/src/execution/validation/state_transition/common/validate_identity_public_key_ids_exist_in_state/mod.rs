use dpp::identifier::Identifier;
use dpp::identity::{IdentityPublicKey, KeyID};
use dpp::validation::ConsensusValidationResult;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use dpp::version::PlatformVersion;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::common::validate_identity_public_key_ids_exist_in_state::v0::validate_identity_public_key_ids_exist_in_state_v0;

pub mod v0;

pub(crate) fn validate_identity_public_key_ids_exist_in_state(
    identity_id: Identifier,
    key_ids: &[KeyID],
    drive: &Drive,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<Vec<IdentityPublicKey>>, Error> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .common_validation_methods
        .validate_identity_public_key_ids_exist_in_state
    {
        0 => validate_identity_public_key_ids_exist_in_state_v0(
            identity_id,
            key_ids,
            drive,
            execution_context,
            transaction,
            platform_version,
        ),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "validate_identity_public_key_ids_exist_in_state".to_string(),
            known_versions: vec![0],
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

    #[test]
    fn should_return_unknown_version_error() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .common_validation_methods
            .validate_identity_public_key_ids_exist_in_state = 99;

        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let identity_id = Identifier::random();
        let mut execution_context = StateTransitionExecutionContext::default_for_platform_version(
            PlatformVersion::latest(),
        )
        .expect("should create execution context");

        let result = validate_identity_public_key_ids_exist_in_state(
            identity_id,
            &[0, 1],
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
                assert_eq!(method, "validate_identity_public_key_ids_exist_in_state");
                assert_eq!(known_versions, vec![0]);
                assert_eq!(received, 99);
            }
            other => panic!("expected UnknownVersionMismatch error, got {:?}", other),
        }
    }
}
