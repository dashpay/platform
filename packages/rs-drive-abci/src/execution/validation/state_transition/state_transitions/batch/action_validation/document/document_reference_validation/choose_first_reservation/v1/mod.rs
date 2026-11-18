use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::{DocumentReferenceLookup, DocumentTypeRef};
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentCreateTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::validation::state_transition::batch::action_validation::document::document_reference_validation::v0::choose_first_scan_budget;

/// Generation 1: generation 0's reservation, and for each competitor the scan may read
/// (`SystemLimits::max_choose_first_candidates`) the double SHA-256 of the create's `validWhen`
/// preimage, which document reference validation 1 bills in full for every competitor it
/// hashes, past the scan's budget too. A competitor's preimage is the writer's with the
/// competitor's owner in the `"$ownerId"` slots, so it hashes as many blocks
/// (see [`super::choose_first_reservation`]).
#[inline(always)]
pub(super) fn choose_first_reservation_v1(
    action: &StateTransitionAction,
    platform_version: &PlatformVersion,
) -> Result<Vec<ValidationOperation>, Error> {
    let StateTransitionAction::BatchAction(batch) = action else {
        return Ok(Vec::new());
    };
    let writer_id = batch.owner_id();
    let competitors = u64::from(platform_version.system_limits.max_choose_first_candidates);
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
        if let Some((_, key)) = lookup.hash_key() {
            if let Ok(preimage) = key.preimage(document_type, None, writer_id, create.data()) {
                reservation.push(ValidationOperation::DoubleSha256(
                    key.function.block_count(preimage.len()),
                ));
            }
        }
        if let Some(choose_first) = &lookup.choose_first {
            let key = &choose_first.valid_when;
            if let Ok(preimage) = key.preimage(document_type, None, writer_id, create.data()) {
                let blocks = key.function.block_count(preimage.len());
                reservation.push(ValidationOperation::DoubleSha256(blocks));
                let mut one_hash = FeeResult::default();
                ValidationOperation::add_many_to_fee_result(
                    &[ValidationOperation::DoubleSha256(blocks)],
                    &mut one_hash,
                    platform_version,
                )?;
                let competitor_hashes =
                    one_hash
                        .processing_fee
                        .checked_mul(competitors)
                        .ok_or(Error::Execution(ExecutionError::Overflow(
                            "chooseFirst competitor hashes reservation overflow",
                        )))?;
                reservation.push(ValidationOperation::PrecalculatedOperation(
                    FeeResult::new_from_processing_fee(competitor_hashes),
                ));
            }
        }
        reservation.push(ValidationOperation::PrecalculatedOperation(
            FeeResult::new_from_processing_fee(choose_first_scan_budget(platform_version)),
        ));
    }
    Ok(reservation)
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
