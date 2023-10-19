use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use dashcore::Transaction;
use platform_version::version::PlatformVersion;

mod v0;

/// Validates asset lock transaction structure
pub fn validate_asset_lock_transaction_structure(
    transaction: &Transaction,
    output_index: usize,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, ProtocolError> {
    match platform_version
        .dpp
        .state_transitions
        .identities
        .asset_locks
        .validate_asset_lock_transaction_structure
    {
        0 => v0::validate_asset_lock_transaction_structure_v0(transaction, output_index),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "validate_asset_lock_transaction_structure".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}
