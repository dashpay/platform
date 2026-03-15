use dpp::identifier::Identifier;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use dpp::version::PlatformVersion;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::common::validate_non_masternode_identity_exists::v0::validate_non_masternode_identity_exists_v0;
mod v0;
pub(in crate::execution::validation) fn validate_non_masternode_identity_exists(
    drive: &Drive,
    identity_id: &Identifier,
    execution_context: &mut StateTransitionExecutionContext,
    tx: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<bool, Error> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .common_validation_methods
        .validate_non_masternode_identity_exists
    {
        0 => validate_non_masternode_identity_exists_v0(
            drive,
            identity_id,
            execution_context,
            tx,
            platform_version,
        ),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "validate_non_masternode_identity_exists".to_string(),
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
            .validate_non_masternode_identity_exists = 99;

        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let identity_id = Identifier::random();
        let mut execution_context = StateTransitionExecutionContext::default_for_platform_version(
            PlatformVersion::latest(),
        )
        .expect("should create execution context");

        let result = validate_non_masternode_identity_exists(
            &platform.drive,
            &identity_id,
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
                assert_eq!(method, "validate_non_masternode_identity_exists");
                assert_eq!(known_versions, vec![0]);
                assert_eq!(received, 99);
            }
            other => panic!("expected UnknownVersionMismatch error, got {:?}", other),
        }
    }
}
