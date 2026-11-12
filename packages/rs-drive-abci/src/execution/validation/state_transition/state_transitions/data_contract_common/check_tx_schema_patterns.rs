//! The pattern bound `check_tx` runs before the non-validating parse of a contract.

use crate::execution::validation::state_transition::ValidationMode;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::data_contract::DataContract;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;

/// In `check_tx` and `recheck_tx`, holds the patterns of `contract`'s schemas to
/// `DataContract::validate_schema_patterns_for_mempool`, returning its consensus error as the
/// parse would. The non-validating parse compiles every pattern and refuses none for its
/// number or its compiled size.
///
/// Mempool only: it keeps out a contract a block may still accept, as that method describes.
pub(in crate::execution) fn validate_document_schemas_patterns_for_check_tx(
    contract: &DataContractInSerializationFormat,
    validation_mode: ValidationMode,
    platform_version: &PlatformVersion,
) -> Result<(), ProtocolError> {
    if !matches!(
        validation_mode,
        ValidationMode::CheckTx | ValidationMode::RecheckTx
    ) {
        return Ok(());
    }
    let result = DataContract::validate_schema_patterns_for_mempool(
        contract.document_schemas(),
        contract.schema_defs(),
        platform_version,
    )?;
    match result.errors.into_iter().next() {
        Some(error) => Err(ProtocolError::ConsensusError(Box::new(error))),
        None => Ok(()),
    }
}
