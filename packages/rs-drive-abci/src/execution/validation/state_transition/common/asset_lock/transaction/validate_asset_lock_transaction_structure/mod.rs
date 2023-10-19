use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::dashcore::Transaction;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

mod v0;

/// Validates asset lock transaction structure
pub fn validate_asset_lock_transaction_structure(
    transaction: &Transaction,
    output_index: usize,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .common_validation_methods
        .asset_locks
        .validate_asset_lock_transaction_structure
    {
        0 => v0::validate_asset_lock_transaction_structure_v0(transaction, output_index),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "validate_asset_lock_transaction_structure".to_string(),
            known_versions: vec![0],
            received: version,
        })),
    }
}
