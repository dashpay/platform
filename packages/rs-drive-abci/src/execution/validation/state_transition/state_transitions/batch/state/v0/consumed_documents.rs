//! Batch-scoped tracking of the documents creates consume.
//!
//! A create revealing a commitment through a `refersTo` lookup with a computed
//! key declaring `consume` deletes the commitment in the same state
//! transition (see `ConsumedDocument`). Every transition of a batch is
//! validated against the same, not yet applied state, and the whole batch is
//! then applied as ONE grove batch, so the create's lookup sees the commitment
//! even when another transition of the batch deletes, replaces, transfers,
//! reprices or buys it, or another create consumes it too. Applied together,
//! the second write would meet a document the first removed and fail block
//! execution rather than refuse the transition.
//!
//! This tracker refuses such a create instead. It records the documents every
//! accepted create of the batch consumes, next to the documents the batch's
//! other document transitions write, and refuses a create consuming any of
//! them (or one document twice) with the `ReferencedEntityNotFoundError` the
//! lookup gives for a commitment that is not there. Nothing is read.
//!
//! Only a protocol version 14 parse produces a lookup that consumes, so the
//! tracker is a no-op for every historical batch. It is also dormant today for
//! a second reason: `max_transitions_in_documents_batch` is 1 at every
//! protocol version, so no batch carries a second transition to collide with,
//! and two transitions in one block apply as two grove batches (the second's
//! lookup sees the first's delete). It keeps consumption safe on the day that
//! cap is raised, as `IndexOnlyBatchEntries` does for indexOnly entries.

use crate::execution::validation::state_transition::batch::action_validation::document::document_reference_validation::ConsumedLookupDocument;
use dpp::consensus::state::document::referenced_entity_not_found_error::ReferencedEntityNotFoundError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::identifier::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use std::collections::BTreeSet;

/// A document of a batch, by contract, document type and id.
type BatchDocument = (Identifier, String, Identifier);

/// The documents one batch's creates consume, and those its other document
/// transitions write. One tracker per batch state validation.
#[derive(Default)]
pub(super) struct ConsumedDocuments {
    /// Written by a transition of the batch other than a create, known before
    /// any transition is validated.
    written: BTreeSet<BatchDocument>,
    /// Consumed by a create of the batch already accepted.
    consumed: BTreeSet<BatchDocument>,
}

impl ConsumedDocuments {
    /// A tracker for a batch of `transitions`: the documents its deletes,
    /// replaces, transfers, repricings and purchases write. A create writes a
    /// document of a new id, which no lookup has found.
    pub(super) fn for_batch(transitions: &[BatchedTransitionAction]) -> Self {
        let written = transitions
            .iter()
            .filter_map(|transition| match transition {
                BatchedTransitionAction::DocumentAction(
                    DocumentTransitionAction::CreateAction(_)
                    | DocumentTransitionAction::IndexOnlyDeleteAction(_),
                ) => None,
                BatchedTransitionAction::DocumentAction(document_transition) => {
                    let base = document_transition.base();
                    Some((
                        base.data_contract_id(),
                        base.document_type_name().clone(),
                        base.id(),
                    ))
                }
                _ => None,
            })
            .collect();
        ConsumedDocuments {
            written,
            consumed: BTreeSet::new(),
        }
    }

    /// Refuses a create of `contract_id` consuming `consumptions` when one of
    /// them is written by another transition of the batch, consumed by a
    /// create it already accepted, or consumed twice by this create. Records
    /// nothing: run it before any tracker records the create, and
    /// [`Self::record`] once every check accepted it, so a refused create
    /// consumes nothing and claims nothing anywhere.
    pub(super) fn validate(
        &self,
        contract_id: Identifier,
        consumptions: &[ConsumedLookupDocument],
    ) -> SimpleConsensusValidationResult {
        let mut claimed = BTreeSet::new();
        for consumption in consumptions {
            let document = (
                contract_id,
                consumption.document.document_type_name.clone(),
                consumption.document.document_id,
            );
            if self.written.contains(&document)
                || self.consumed.contains(&document)
                || !claimed.insert(document)
            {
                return SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(
                        ReferencedEntityNotFoundError::new(
                            consumption.referenced_id,
                            consumption.reference_target.clone(),
                            consumption.path.clone(),
                        ),
                    )),
                );
            }
        }
        SimpleConsensusValidationResult::new()
    }

    /// Records the documents an accepted create of `contract_id` consumes, for
    /// the rest of the batch.
    pub(super) fn record(
        &mut self,
        contract_id: Identifier,
        consumptions: &[ConsumedLookupDocument],
    ) {
        self.consumed.extend(consumptions.iter().map(|consumption| {
            (
                contract_id,
                consumption.document.document_type_name.clone(),
                consumption.document.document_id,
            )
        }));
    }

    /// [`Self::validate`], then [`Self::record`] when it holds.
    #[cfg(test)]
    fn validate_and_record(
        &mut self,
        contract_id: Identifier,
        consumptions: &[ConsumedLookupDocument],
    ) -> SimpleConsensusValidationResult {
        let result = self.validate(contract_id, consumptions);
        if result.is_valid() {
            self.record(contract_id, consumptions);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::DocumentPropertyReferenceTarget;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use dpp::version::PlatformVersion;
    use drive::drive::contract::DataContractFetchInfo;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};
    use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::ConsumedDocument;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionV0;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
    use std::sync::Arc;

    fn contract() -> Arc<DataContractFetchInfo> {
        Arc::new(DataContractFetchInfo::dpns_contract_fixture(
            PlatformVersion::latest().protocol_version,
        ))
    }

    fn consumption(document_id: [u8; 32]) -> ConsumedLookupDocument {
        ConsumedLookupDocument {
            document: ConsumedDocument {
                document_id: Identifier::from(document_id),
                document_type_name: "preorder".to_string(),
            },
            referenced_id: Identifier::from([9; 32]),
            reference_target: DocumentPropertyReferenceTarget::Identity,
            path: "$creatorId".to_string(),
        }
    }

    fn delete_of(
        contract: &Arc<DataContractFetchInfo>,
        document_id: [u8; 32],
    ) -> BatchedTransitionAction {
        BatchedTransitionAction::DocumentAction(DocumentTransitionAction::DeleteAction(
            DocumentDeleteTransitionAction::V0(DocumentDeleteTransitionActionV0 {
                base: DocumentBaseTransitionAction::V0(DocumentBaseTransitionActionV0 {
                    id: Identifier::from(document_id),
                    identity_contract_nonce: 1,
                    document_type_name: "preorder".to_string(),
                    data_contract: contract.clone(),
                    token_cost: None,
                    shielded_token_payment: None,
                    gas_fees_paid_by: GasFeesPaidBy::default(),
                    contract_gas_fees_paid_by: GasFeesPaidBy::default(),
                    declared_action_fee: None,
                }),
            }),
        ))
    }

    fn refused(result: &SimpleConsensusValidationResult) -> bool {
        matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(e))]
                if e.path() == "$creatorId"
        )
    }

    #[test]
    fn should_refuse_a_create_consuming_what_another_write_of_the_batch_touches() {
        let contract = contract();
        let contract_id = contract.contract.id();
        let mut consumed = ConsumedDocuments::for_batch(&[delete_of(&contract, [1; 32])]);

        // The batch deletes it too
        assert!(refused(
            &consumed.validate_and_record(contract_id, &[consumption([1; 32])])
        ));
        // A refused create claims nothing, so the next one may consume what it
        // listed beside the refused document
        assert!(refused(&consumed.validate_and_record(
            contract_id,
            &[consumption([2; 32]), consumption([1; 32])]
        )));
        assert!(consumed
            .validate_and_record(contract_id, &[consumption([2; 32])])
            .is_valid());
        // Another create of the batch consumed it already
        assert!(refused(
            &consumed.validate_and_record(contract_id, &[consumption([2; 32])])
        ));
        // One create consuming one document twice
        assert!(refused(&consumed.validate_and_record(
            contract_id,
            &[consumption([3; 32]), consumption([3; 32])]
        )));
        // The same id in another contract is another document
        assert!(consumed
            .validate_and_record(Identifier::from([7; 32]), &[consumption([2; 32])])
            .is_valid());
    }

    #[test]
    fn should_claim_nothing_until_the_accepted_create_is_recorded() {
        let contract = contract();
        let contract_id = contract.contract.id();
        let mut consumed = ConsumedDocuments::for_batch(&[]);

        // The check alone claims nothing: a create a later tracker refuses
        // leaves the document free for the rest of the batch
        assert!(consumed
            .validate(contract_id, &[consumption([4; 32])])
            .is_valid());
        assert!(consumed
            .validate(contract_id, &[consumption([4; 32])])
            .is_valid());
        // Recorded once every check accepted the create
        consumed.record(contract_id, &[consumption([4; 32])]);
        assert!(refused(
            &consumed.validate(contract_id, &[consumption([4; 32])])
        ));
    }
}
