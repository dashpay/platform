use dpp::data_contract::document_type::{DocumentReferenceLookup, DocumentTypeRef};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentCreateTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use drive::state_transition_action::StateTransitionAction;

use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::validation::state_transition::batch::action_validation::document::document_reference_validation::v0::choose_first_scan_budget;

/// Generation 0: for every create of a document type declaring `chooseFirst`, the double
/// SHA-256s of its `findBy` and `validWhen` preimages and the competitor scan's whole budget
/// (see [`super::choose_first_reservation`]).
#[inline(always)]
pub(super) fn choose_first_reservation_v0(
    action: &StateTransitionAction,
    platform_version: &PlatformVersion,
) -> Vec<ValidationOperation> {
    let StateTransitionAction::BatchAction(batch) = action else {
        return Vec::new();
    };
    let writer_id = batch.owner_id();
    let mut reservation = Vec::new();
    for transition in batch.transitions() {
        let BatchedTransitionAction::DocumentAction(DocumentTransitionAction::CreateAction(create)) =
            transition
        else {
            continue;
        };
        let base = create.base();
        let Ok(document_type) = base
            .data_contract_fetch_info_ref()
            .contract
            .document_type_for_name(base.document_type_name())
        else {
            continue;
        };
        let Some(lookup) = choose_first_lookup(document_type) else {
            continue;
        };
        // Neither function reads `"."` beside `chooseFirst`, which registration refuses
        for key in lookup.hash_key().map(|(_, key)| key).into_iter().chain(
            lookup
                .choose_first
                .as_ref()
                .map(|choose_first| &choose_first.valid_when),
        ) {
            if let Ok(preimage) = key.preimage(document_type, None, writer_id, create.data()) {
                reservation.push(ValidationOperation::DoubleSha256(
                    key.function.block_count(preimage.len()),
                ));
            }
        }
        reservation.push(ValidationOperation::PrecalculatedOperation(
            FeeResult::new_from_processing_fee(choose_first_scan_budget(platform_version)),
        ));
    }
    reservation
}

/// The lookup through which `document_type` chooses the first of the commitments to a
/// create's values (`chooseFirst`), if it declares one.
fn choose_first_lookup(document_type: DocumentTypeRef<'_>) -> Option<&DocumentReferenceLookup> {
    document_type
        .reference_declarations()
        .filter_map(|(_, reference)| reference.target())
        .flat_map(|target| target.leaves())
        .filter_map(|leaf| leaf.as_any_document_reference())
        .find_map(|declaration| {
            declaration
                .lookup
                .filter(|lookup| lookup.choose_first.is_some())
        })
}
