use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::common::validate_addresses_for_balances_and_nonces::v0::validate_addresses_for_balances_and_nonces_v0;
use dpp::fee::Credits;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use std::collections::BTreeMap;
use dpp::address_funds::PlatformAddress;

pub mod v0;

pub(crate) fn validate_addresses_for_balances_and_nonces(
    minimum_balances: &BTreeMap<PlatformAddress, Credits>,
    drive: &Drive,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<()>, Error> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .common_validation_methods
        .validate_addresses_for_balances_and_nonces
    {
        0 => validate_addresses_for_balances_and_nonces_v0(
            minimum_balances,
            drive,
            execution_context,
            transaction,
            platform_version,
        ),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "validate_addresses_for_balances_and_nonces".to_string(),
            known_versions: vec![0],
            received: version,
        })),
    }
}
