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
///
/// The block checks the schema with its property type shorthands rewritten into the long form
/// they stand for (`DocumentType::expand_property_type_shorthands`, protocol version 14), and the
/// check resolves every `$ref`, so a reference to a keyword only the long form writes resolves
/// there. This checks the same rewritten schema and definitions. Without full validation the
/// rewrite refuses nothing: a shorthand the block refuses is left as sent here.
pub(in crate::execution) fn validate_document_schemas_depth_for_check_tx(
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
    let depth_check_version = platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .validate_max_depth;
    if depth_check_version < 1 {
        return Ok(());
    }

    let expanded_schema_defs = contract
        .schema_defs()
        .map(|defs| {
            DocumentType::expand_schema_defs_property_type_shorthands(defs, false, platform_version)
        })
        .transpose()?
        .flatten();
    let schema_defs = match expanded_schema_defs {
        Some(defs) => Some(Value::from(defs)),
        None => contract.schema_defs().map(|defs| Value::from(defs.clone())),
    };
    for schema in contract.document_schemas().values() {
        let schema =
            DocumentType::expand_property_type_shorthands(schema, false, platform_version)?
                .unwrap_or_else(|| schema.clone());
        let root_schema =
            DocumentType::enrich_with_base_schema(schema, schema_defs.clone(), platform_version)?;
        let result = validate_max_depth(&root_schema, platform_version)?;
        if let Some(error) = result.errors.into_iter().next() {
            return Err(ProtocolError::ConsensusError(Box::new(error)));
        }
    }
    Ok(())
}
