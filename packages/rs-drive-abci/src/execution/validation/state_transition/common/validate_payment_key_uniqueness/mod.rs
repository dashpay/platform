use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::common::validate_payment_key_uniqueness::v0::validate_payment_key_uniqueness_in_state_v0;
use dpp::identity::KeyID;
use dpp::platform_value::Identifier;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

pub mod v0;

/// Validates that adding the given keys leaves the identity with at most one
/// active key of each DIP-33 payment purpose (`PAYMENT_SCAN`, `PAYMENT_SPEND`),
/// taking keys disabled in the same transition into account.
#[allow(clippy::too_many_arguments)]
pub(crate) fn validate_payment_key_uniqueness_in_state(
    identity_id: Identifier,
    public_keys_being_added: &[IdentityPublicKeyInCreation],
    public_key_ids_to_disable: &[KeyID],
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
        .validate_payment_key_uniqueness
    {
        0 => validate_payment_key_uniqueness_in_state_v0(
            identity_id,
            public_keys_being_added,
            public_key_ids_to_disable,
            drive,
            execution_context,
            transaction,
            platform_version,
        ),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "validate_payment_key_uniqueness_in_state".to_string(),
            known_versions: vec![0],
            received: version,
        })),
    }
}
