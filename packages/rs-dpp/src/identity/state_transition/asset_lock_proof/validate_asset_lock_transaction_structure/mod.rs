use crate::validation::ConsensusValidationResult;
use crate::ProtocolError;
use dashcore::{Transaction, TxOut};
use platform_version::version::PlatformVersion;

mod v0;

/// Validates asset lock transaction structure
pub fn validate_asset_lock_transaction_structure(
    transaction: &Transaction,
    output_index: u32,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<TxOut>, ProtocolError> {
    match platform_version
        .dpp
        .state_transitions
        .identities
        .asset_locks
        .validate_asset_lock_transaction_structure
    {
        0 => Ok(v0::validate_asset_lock_transaction_structure_v0(
            transaction,
            output_index,
        )),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "validate_asset_lock_transaction_structure".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}
