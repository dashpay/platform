use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::consensus::state::data_trigger::data_trigger_condition_error::DataTriggerConditionError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;

use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::ProtocolError;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentCreateTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::DocumentReplaceTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;

use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContextMethodsV0;
use crate::execution::validation::state_transition::batch::data_triggers::{
    DataTriggerExecutionContext, DataTriggerExecutionResult,
};
use dpp::version::PlatformVersion;

/// DIP-33 profile payment address fields, storage form: type byte `0x00`
/// (P2PKH) or `0x01` (P2SH) followed by a 20-byte HASH160.
const PAYMENT_ADDRESS_FIELDS: [&str; 2] = ["corePaymentAddress", "platformPaymentAddress"];

/// Validates the DIP-33 payment address fields of a DashPay profile document
/// on creation and replacement.
///
/// The schema constrains the fields to exactly 21 bytes; this trigger enforces
/// the remaining invariant the schema vocabulary cannot express: the leading
/// type byte must be `0x00` (P2PKH) or `0x01` (P2SH). Unlike a Base58Check
/// string, the storage form carries no checksum, so this check makes an
/// accepted value fully decodable as the advertised address type.
#[inline(always)]
pub(super) fn validate_profile_payment_addresses_data_trigger_v0(
    document_transition: &DocumentTransitionAction,
    context: &mut DataTriggerExecutionContext<'_>,
    _platform_version: &PlatformVersion,
) -> Result<DataTriggerExecutionResult, Error> {
    let data_contract_fetch_info = document_transition.base().data_contract_fetch_info();
    let data_contract = &data_contract_fetch_info.contract;
    let mut result = DataTriggerExecutionResult::default();

    if context.state_transition_execution_context.in_dry_run() {
        return Ok(result);
    }

    let data = match document_transition {
        DocumentTransitionAction::CreateAction(create_transition) => create_transition.data(),
        DocumentTransitionAction::ReplaceAction(replace_transition) => replace_transition.data(),
        _ => {
            return Err(Error::Execution(ExecutionError::DataTriggerExecutionError(
                format!(
                    "the Document Transition {} isn't 'CREATE' or 'REPLACE'",
                    document_transition.base().id()
                ),
            )));
        }
    };

    for field in PAYMENT_ADDRESS_FIELDS {
        let Some(address_bytes) = data
            .get_optional_binary_bytes(field)
            .map_err(ProtocolError::ValueError)?
        else {
            continue;
        };

        if !matches!(address_bytes.first(), Some(0x00) | Some(0x01)) {
            let err = DataTriggerConditionError::new(
                data_contract.id(),
                document_transition.base().id(),
                format!(
                    "{field} must start with address type byte 0x00 (P2PKH) or 0x01 (P2SH), got {}",
                    address_bytes
                        .first()
                        .map(|byte| format!("0x{byte:02x}"))
                        .unwrap_or_else(|| "an empty value".to_string()),
                ),
            );

            result.add_error(err);

            return Ok(result);
        }
    }

    Ok(result)
}
