use std::collections::BTreeMap;

use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::{PropertyReference, ReferenceHolder};
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
use crate::execution::types::execution_operation::ValidationOperation;

/// Generation 0: for every lookup leaf of every reference a create's document type declares
/// (each leaf of an `anyOf` or `allOf` counted, as any of them may be checked), and each value
/// it is checked for (the property's value, each element of a typed array, the writer for
/// `ownerRefersTo` and `creatorRefersTo`), the double SHA-256 of its computed key by the blocks
/// the create's values make it (but for a `chooseFirst` lookup, whose `findBy` hash
/// `choose_first_reservation` reserves), `lookup_query_credits_reserved`, and for a lookup
/// declaring `consume` Drive's estimated delete of a document of the consumed type
/// (see [`super::reference_lookup_reservation`]). An unset property makes no lookup, and a
/// key whose preimage cannot be assembled is refused by structure validation before any read.
#[inline(always)]
pub(super) fn reference_lookup_reservation_v0(
    action: &StateTransitionAction,
    drive: &Drive,
    block_info: &BlockInfo,
    platform_version: &PlatformVersion,
) -> Result<Vec<ValidationOperation>, Error> {
    let StateTransitionAction::BatchAction(batch) = action else {
        return Ok(Vec::new());
    };
    let writer_id = batch.owner_id();
    let query_allowance = platform_version
        .fee_version
        .document_reference
        .lookup_query_credits_reserved;
    // The delete of a consumed document is priced once for each consumed type
    let mut consumed_delete_fees: BTreeMap<&str, Credits> = BTreeMap::new();
    let mut reservation = Vec::new();
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
            for declaration in target
                .leaves()
                .into_iter()
                .filter_map(|leaf| leaf.as_any_document_reference())
            {
                let Some(lookup) = declaration.lookup else {
                    continue;
                };
                let mut operations = Vec::new();
                // A key's `"."` reads the identifier the reference holds, any of which is as
                // long as the writer's id, or names the property carrying the reference
                let reference_id = reads_an_identifier.then_some(writer_id);
                if let (None, Some((_, key))) = (&lookup.choose_first, lookup.hash_key()) {
                    if let Ok(preimage) = key.preimage(document_type, reference_id, writer_id, data)
                    {
                        operations.push(ValidationOperation::DoubleSha256(
                            key.function.block_count(preimage.len()),
                        ));
                    }
                }
                operations.push(ValidationOperation::PrecalculatedOperation(
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
                        operations.push(ValidationOperation::PrecalculatedOperation(
                            FeeResult::new_from_processing_fee(delete_fee),
                        ));
                    }
                }
                for _ in 0..values {
                    reservation.extend(operations.iter().cloned());
                }
            }
        }
    }
    Ok(reservation)
}

/// How many values a create's reference held by `holder` is checked for: one for the writer
/// (`ownerRefersTo`, `creatorRefersTo`), one for a set property, each element of a set typed
/// array when the reference is on its `items`, and none for an unset property.
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
        Ok(Some(Value::Array(elements))) if on_elements => elements.len(),
        Ok(Some(_)) => usize::from(!on_elements),
    }
}
