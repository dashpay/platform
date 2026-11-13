//! The pattern bound `check_tx` holds a contract to before anything compiles its patterns.

use dpp::data_contract::DataContract;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::StateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;

/// Holds the patterns of the contract a create or update carries to
/// `DataContract::validate_schema_patterns_for_mempool`. Any other transition passes.
///
/// `check_tx` runs this before the identity balance pre-check and `recheck_tx` before the
/// transformer. Before protocol version 15 the pre-check estimates the registration fee by
/// parsing every index, which compiles each `regexPattern` of a contested index, and at every
/// version the transformer's non-validating parse compiles every pattern. Neither refuses a
/// pattern for how many there are or how large it compiles.
///
/// Mempool only: it keeps out a contract a block may still accept, as that method describes.
pub(in crate::execution) fn validate_contract_schema_patterns_for_check_tx(
    state_transition: &StateTransition,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, ProtocolError> {
    let contract = match state_transition {
        StateTransition::DataContractCreate(create) => create.data_contract(),
        StateTransition::DataContractUpdate(update) => update.data_contract(),
        _ => return Ok(SimpleConsensusValidationResult::new()),
    };
    DataContract::validate_schema_patterns_for_mempool(
        contract.document_schemas(),
        contract.schema_defs(),
        platform_version,
    )
}
