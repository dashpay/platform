//! The schema depth check `check_tx` runs before the non-validating parse of a contract.

use crate::execution::validation::state_transition::ValidationMode;
use dpp::data_contract::document_type::schema::validate_max_depth;
use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;

/// In `check_tx` and `recheck_tx`, runs the schema depth check over every document type of
/// `contract`, returning its first consensus error as the parse would. The non-validating parse
/// skips the check, and a `$defs` graph reaching one definition along many paths expands
/// exponentially there.
///
/// Mempool only: block execution runs the same check first under full validation, so this
/// refuses nothing a block accepts. Generation 0 of the check is itself exponential on such a
/// graph, so it runs from generation 1 only.
pub(in crate::execution) fn validate_document_schemas_depth_for_check_tx(
    contract: &DataContractInSerializationFormat,
    validation_mode: ValidationMode,
    platform_version: &PlatformVersion,
) -> Result<(), ProtocolError> {
    if !matches!(
        validation_mode,
        ValidationMode::CheckTx | ValidationMode::RecheckTx
    ) || platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .validate_max_depth
        < 1
    {
        return Ok(());
    }

    let schema_defs = contract.schema_defs().map(|defs| Value::from(defs.clone()));
    for schema in contract.document_schemas().values() {
        let root_schema = DocumentType::enrich_with_base_schema(
            schema.clone(),
            schema_defs.clone(),
            platform_version,
        )?;
        let mut result = validate_max_depth(&root_schema, platform_version)?;
        if !result.is_valid() {
            return Err(ProtocolError::ConsensusError(Box::new(
                result.errors.remove(0),
            )));
        }
    }
    Ok(())
}
