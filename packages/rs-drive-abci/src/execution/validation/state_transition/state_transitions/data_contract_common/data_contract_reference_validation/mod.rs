mod v0;

use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::query::TransactionArg;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;

/// Validates the `refersTo` reference declarations carried by a contract's
/// document types, at contract create or update time.
///
/// Only `permanentDocument` targets carry declaration content to check: the
/// referenced contract (the declaring contract itself when no contract id is
/// named) must contain the referenced document type, and that type must forbid
/// deletion. Identity, contract and token targets declare nothing beyond their
/// kind, so they have nothing to validate here.
pub(in crate::execution::validation::state_transition) fn validate_data_contract_references(
    contract: &DataContract,
    drive: &Drive,
    block_info: &BlockInfo,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .data_contract_reference_validation
    {
        0 => v0::validate_data_contract_references_v0(
            contract,
            drive,
            block_info,
            execution_context,
            transaction,
            platform_version,
        ),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "validate_data_contract_references".to_string(),
            known_versions: vec![0],
            received: version,
        })),
    }
}
