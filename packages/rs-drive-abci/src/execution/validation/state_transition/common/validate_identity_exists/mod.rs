use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::common::validate_identity_exists::v0::validate_identity_exists_v0;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
mod v0;
pub(in crate::execution::validation) fn validate_identity_exists(
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
        .validate_identity_exists
    {
        0 => {
            validate_identity_exists_v0(drive, identity_id, execution_context, tx, platform_version)
        }
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "validate_identity_exists".to_string(),
            known_versions: vec![0],
            received: version,
        })),
    }
}
