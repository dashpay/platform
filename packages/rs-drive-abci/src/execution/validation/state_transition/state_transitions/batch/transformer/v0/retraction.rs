use crate::execution::validation::state_transition::batch::action_validation::document::document_replace_transition_action::state_v1::ReplaceConditionInputs;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::DataContract;
use dpp::platform_value::Identifier;
use dpp::prelude::ConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::DocumentReplaceTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;

/// Judges the replace in `result` of an owner under `bar`, a ban or a live suspension the
/// moderation gate found, which let the replace through only as a retraction: the document it
/// writes must meet its type's `retractedWhen`. Returns the bar to refuse the replace with when
/// it does not, or when the condition cannot be evaluated (an overflow, a division by zero), so
/// that a fault keeps the bar rather than lifting it. `None` when it does, and for a replace
/// already refused, whose own refusal stands.
///
/// The condition is judged as an `immutable` entry's is, on the document the replace writes (its
/// `$updatedAt` this block's time), with the stored one under `$old`. Every other rule of the
/// type still judges the replace afterwards. Judged in the transformer, after the stored
/// document is fetched, so the mempool refuses a barred owner's replace as a block does. Added in
/// place to the shipped transformer at protocol version 14 and inert before it: the gate, and so
/// a bar, exist only from protocol version 14.
pub(super) fn judge_barred_replace(
    contract: &DataContract,
    result: &ConsensusValidationResult<BatchedTransitionAction>,
    owner_id: Identifier,
    bar: &ConsensusError,
) -> Option<ConsensusError> {
    if !result.is_valid() {
        return None;
    }
    let Some(BatchedTransitionAction::DocumentAction(DocumentTransitionAction::ReplaceAction(
        action,
    ))) = result.data.as_ref()
    else {
        return None;
    };
    let retracts = contract
        .document_type_optional_for_name(action.base().document_type_name())
        .and_then(|document_type| {
            document_type.retracted_when().map(|condition| {
                condition
                    .holds(
                        &action.condition_data(),
                        &action.condition_system_values(owner_id),
                    )
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    (!retracts).then(|| bar.clone())
}
