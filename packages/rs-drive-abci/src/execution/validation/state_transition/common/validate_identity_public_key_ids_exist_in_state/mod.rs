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
