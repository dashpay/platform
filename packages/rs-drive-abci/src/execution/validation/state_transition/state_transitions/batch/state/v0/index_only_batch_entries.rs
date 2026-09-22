//! Batch-scoped tracking of the index entries indexOnly creates write.
//!
//! Every transition of a batch is validated against the same, not yet
//! applied state, and the whole batch is then applied as ONE grove batch.
//! For a stored document type that is safe: two creates in one batch
//! cannot share an id (`find_duplicates_by_id` refuses that at basic
//! structure) and so write distinct primary rows. An indexOnly document
//! has no row — its index entries ARE the rows — and two creates by the
//! same owner can address the same entry under one index while differing
//! under another (a shorter index projects fewer properties). The create's
//! state probe (`has_index_only_document_entry`) reads committed state, so
//! it sees neither create's entries; the storage walker's if-not-exists
//! insert reads the same state; and grovedb files a batch's operations by
//! path and key, so the second insert silently replaces the first. The
//! result is one entry carrying the other document's row commitment while
//! the rest of the loser's entries stand: a document nobody can delete
//! (its commitment probe fails on the replaced entry) or recreate (its
//! surviving entries are duplicates).
//!
//! This tracker closes the gap the state probe cannot see. It records the
//! entries every create the batch has already accepted will write and
//! refuses a later create of the same batch that would write any of them,
//! with the `DuplicateUniqueIndexError` the state probe raises for the
//! same collision against committed state. Nothing is read: the entry
//! paths and member keys come from the action's own values through
//! [`Drive::index_only_entry_paths_and_key`], the derivation the index
//! walkers write with, so nothing is billed and the check cannot drift
//! from storage.
//!
//! `index_only()` can only be true on a PV14+ contract, so the tracker is
//! a no-op for every historical batch. It is also dormant today for a
//! second reason: `max_transitions_in_documents_batch` is 1 at every
//! protocol version, so basic structure validation refuses any batch
//! carrying two transitions before either reaches this loop, and two
//! transitions in one block apply as two grove batches (the second's
//! state probe sees the first's entries). The tracker is what keeps
//! indexOnly types safe on the day that cap is raised — the test in
//! `batch/tests/document/index_only.rs` drives the loop directly to pin
//! it.

use crate::error::Error;
use dpp::consensus::basic::document::InvalidDocumentTypeError;
use dpp::consensus::state::document::duplicate_unique_index_error::DuplicateUniqueIndexError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::document::Document;
use dpp::identifier::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{
    DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0,
    DocumentFromCreateTransitionAction,
};
use std::collections::BTreeSet;

/// The `(entry path, member key)` pairs every accepted indexOnly create of
/// one batch writes. One tracker per batch state validation.
#[derive(Default)]
pub(super) struct IndexOnlyBatchEntries {
    entries: BTreeSet<(Vec<Vec<u8>>, Vec<u8>)>,
}

impl IndexOnlyBatchEntries {
    /// Refuses `create_action` when any entry it would write is already
    /// claimed by an earlier accepted create of the same batch, and claims
    /// all of its entries otherwise. Call it only for a create that state
    /// validation (and the data triggers) accepted: a refused create writes
    /// nothing, so it must not block a later create in the batch. A no-op
    /// for stored (non-indexOnly) document types.
    pub(super) fn validate_and_record_create(
        &mut self,
        create_action: &DocumentCreateTransitionAction,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let contract_fetch_info = create_action.base().data_contract_fetch_info();
        let contract = &contract_fetch_info.contract;
        let document_type_name = create_action.base().document_type_name();

        // The create's own state validation resolves the document type
        // first and refuses an unknown one, so this mirrors that refusal
        // rather than treating it as a code error.
        let Some(document_type) = contract.document_type_optional_for_name(document_type_name)
        else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(document_type_name.clone(), contract.id()).into(),
            ));
        };

        if !document_type.index_only() {
            return Ok(SimpleConsensusValidationResult::new());
        }

        let document =
            Document::try_from_create_transition_action(create_action, owner_id, platform_version)?;

        let mut claimed = Vec::new();
        for index in document_type.indexes().values() {
            let (paths, member_key) = Drive::index_only_entry_paths_and_key(
                contract.id(),
                document_type,
                index,
                &document,
                platform_version,
            )
            .map_err(Error::Drive)?;
            for path in paths {
                let entry = (path, member_key.clone());
                if self.entries.contains(&entry) {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        DuplicateUniqueIndexError::new(
                            create_action.base().id(),
                            index
                                .properties
                                .iter()
                                .map(|property| property.name.clone())
                                .chain(index.terminal.clone())
                                .collect(),
                        )
                        .into(),
                    ));
                }
                claimed.push(entry);
            }
        }

        // Claim only once every entry is known to be free, so a refused
        // create leaves the tracker exactly as it found it.
        self.entries.extend(claimed);

        Ok(SimpleConsensusValidationResult::new())
    }
}
