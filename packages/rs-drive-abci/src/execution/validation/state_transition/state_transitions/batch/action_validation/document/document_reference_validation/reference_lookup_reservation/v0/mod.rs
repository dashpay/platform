use std::collections::{BTreeMap, BTreeSet};

use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::{
    DocumentPropertyReferenceTarget, PropertyReference, ReferenceCombinator, ReferenceHolder,
};
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;
use dpp::platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentCreateTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use drive::state_transition_action::StateTransitionAction;

use crate::error::Error;
use crate::execution::types::check_tx_reservation::CheckTxReservation;
use crate::execution::types::execution_operation::ValidationOperation;

/// Generation 0: for every lookup leaf of every reference a create's document type declares
/// (each leaf of an `anyOf` or `allOf` counted, as any of them may be checked), and each value
/// it is checked for (the property's value, each distinct identifier of a typed array, as a
/// block checks a repeated element once, the writer for `ownerRefersTo` and `creatorRefersTo`),
/// the double SHA-256 of its computed key by the blocks the create's values make it (but for a
/// `chooseFirst` lookup, whose `findBy` hash `choose_first_reservation` reserves),
/// `lookup_query_credits_reserved`, and for a lookup declaring `consume` Drive's estimated
/// delete of a document of the consumed type (see [`super::reference_lookup_reservation`]).
/// The allowance is a bound; the hash and the delete are billed for a leaf a block checks
/// whenever the write holds, and bounds for one it checks only when an earlier `anyOf` operand
/// fails ([`leaves_checked_when_the_write_holds`]). An unset property makes no lookup, and a key
/// whose preimage cannot be assembled is refused by structure validation before any read.
#[inline(always)]
pub(super) fn reference_lookup_reservation_v0(
    action: &StateTransitionAction,
    drive: &Drive,
    block_info: &BlockInfo,
    platform_version: &PlatformVersion,
) -> Result<CheckTxReservation, Error> {
    let StateTransitionAction::BatchAction(batch) = action else {
        return Ok(CheckTxReservation::default());
    };
    let writer_id = batch.owner_id();
    let query_allowance = platform_version
        .fee_version
        .document_reference
        .lookup_query_credits_reserved;
    let mut reservation = CheckTxReservation::default();
    for transition in batch.transitions() {
        let BatchedTransitionAction::DocumentAction(DocumentTransitionAction::CreateAction(create)) =
            transition
        else {
            continue;
        };
        let base = create.base();
        let contract = &base.data_contract_fetch_info_ref().contract;
        let Ok(document_type) = contract.document_type_for_name(base.document_type_name()) else {
            continue;
        };
        let data = create.data();
        // The delete of a document this create consumes is priced once for each consumed type
        // of its contract
        let mut consumed_delete_fees: BTreeMap<&str, Credits> = BTreeMap::new();
        for (holder, reference) in document_type.reference_declarations() {
            let (target, values, reads_an_identifier) = match reference {
                PropertyReference::KeyId(_) => continue,
                PropertyReference::Value(target) => {
                    (target, checked_values(&holder, data, false), true)
                }
                PropertyReference::Elements { target, .. } => {
                    (target, checked_values(&holder, data, true), true)
                }
                PropertyReference::Revealed(target) => {
                    (target, checked_values(&holder, data, false), false)
                }
            };
            if values == 0 {
                continue;
            }
            for (leaf, checked_when_held) in leaves_checked_when_the_write_holds(target) {
                let Some(declaration) = leaf.as_any_document_reference() else {
                    continue;
                };
                let Some(lookup) = declaration.lookup else {
                    continue;
                };
                let mut operations = CheckTxReservation::default();
                let mut billed_or_bound = Vec::new();
                // A key's `"."` reads the identifier the reference holds, any of which is as
                // long as the writer's id, or names the property carrying the reference
                let reference_id = reads_an_identifier.then_some(writer_id);
                if let (None, Some((_, key))) = (&lookup.choose_first, lookup.hash_key()) {
                    if let Ok(preimage) = key.preimage(document_type, reference_id, writer_id, data)
                    {
                        billed_or_bound.push(ValidationOperation::DoubleSha256(
                            key.function.block_count(preimage.len()),
                        ));
                    }
                }
                operations
                    .bounds
                    .push(ValidationOperation::PrecalculatedOperation(
                        FeeResult::new_from_processing_fee(query_allowance),
                    ));
                // Registration admits `consume` only beside a `findBy` into the declaring
                // contract
                if lookup.consume {
                    let delete_fee = match consumed_delete_fees.get(declaration.document_type_name)
                    {
                        Some(delete_fee) => Some(*delete_fee),
                        None => {
                            match contract.document_type_for_name(declaration.document_type_name) {
                                Ok(consumed_type) => {
                                    let delete_fee = drive
                                        .consumed_document_delete_cost(
                                            contract,
                                            consumed_type,
                                            block_info,
                                            platform_version,
                                        )
                                        .map_err(Error::Drive)?
                                        .processing_fee;
                                    consumed_delete_fees
                                        .insert(declaration.document_type_name, delete_fee);
                                    Some(delete_fee)
                                }
                                Err(_) => None,
                            }
                        }
                    };
                    if let Some(delete_fee) = delete_fee {
                        billed_or_bound.push(ValidationOperation::PrecalculatedOperation(
                            FeeResult::new_from_processing_fee(delete_fee),
                        ));
                    }
                }
                if checked_when_held {
                    operations.billed.extend(billed_or_bound);
                } else {
                    operations.bounds.extend(billed_or_bound);
                }
                for _ in 0..values {
                    reservation.extend(operations.clone());
                }
            }
        }
    }
    Ok(reservation)
}

/// How many values a create's reference held by `holder` is checked for: one for the writer
/// (`ownerRefersTo`, `creatorRefersTo`), one for a set property, each distinct identifier of a
/// set typed array when the reference is on its `items` (a block reads each element as a
/// 32-byte identifier and skips one an earlier element repeats), and none for an unset
/// property.
fn checked_values(
    holder: &ReferenceHolder,
    data: &BTreeMap<String, Value>,
    on_elements: bool,
) -> usize {
    let ReferenceHolder::Property(path) = holder else {
        return 1;
    };
    match data.get_optional_at_path(path) {
        Ok(None) | Ok(Some(Value::Null)) | Err(_) => 0,
        Ok(Some(Value::Array(elements))) if on_elements => elements
            .iter()
            .filter_map(|element| element.to_hash256().ok())
            .collect::<BTreeSet<[u8; 32]>>()
            .len(),
        Ok(Some(_)) => usize::from(!on_elements),
    }
}

/// The single targets of `target` in declared order, each with whether a block checks it
/// whenever the write holds: every operand of an `allOf`, and the first of an `anyOf`, which
/// stops at the first operand that holds. A later `anyOf` operand is checked only when every
/// earlier one fails.
fn leaves_checked_when_the_write_holds(
    target: &DocumentPropertyReferenceTarget,
) -> Vec<(&DocumentPropertyReferenceTarget, bool)> {
    fn walk<'a>(
        target: &'a DocumentPropertyReferenceTarget,
        checked_when_held: bool,
        leaves: &mut Vec<(&'a DocumentPropertyReferenceTarget, bool)>,
    ) {
        match target.combinator() {
            None => leaves.push((target, checked_when_held)),
            Some((combinator, operands)) => {
                for (index, operand) in operands.operands().iter().enumerate() {
                    let checked = checked_when_held
                        && (combinator == ReferenceCombinator::AllOf || index == 0);
                    walk(operand, checked, leaves);
                }
            }
        }
    }
    let mut leaves = Vec::new();
    walk(target, true, &mut leaves);
    leaves
}
