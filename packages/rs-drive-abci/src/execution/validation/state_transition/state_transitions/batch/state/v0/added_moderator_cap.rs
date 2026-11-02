//! The cap on the members the leader of a seated moderation team adds after the election.
//!
//! An `addedModerator` of the moderation charters system contract names a seated
//! `electedCharter` and a member the leader adds from the proposal's join requests. The target
//! contract's elected declaration caps them: at most `maxAddedModerators` per charter at a time,
//! counting the additions that exist now (the leader takes one back by deleting it, which frees
//! its slot).
//! The schema can not count documents, so this is a consensus rule of the document create,
//! judged here once the create's own state validation passed: its references then proved the
//! charter is seated, that its leader is the writer, and that the member asked to join.
//!
//! It is state validation, which check tx does not run for a batch, so an addition over the
//! cap is admitted to the mempool and refused, paid, in the block, as a unique index conflict
//! is: counting the additions takes reads the batch transformer does not make.
//!
//! The additions are counted in committed state, plus those an earlier create of the same batch
//! was accepted for: the batch applies as one grove batch, so none of its creates is in state
//! yet. That second half is dormant while `max_transitions_in_documents_batch` is 1, as it is at
//! every protocol version; two additions in two batches of one block are counted from state,
//! since the second batch is validated after the first applied.
//!
//! Only a create of the charter contract's `addedModerator` takes this path. The charter
//! contract exists in state from protocol version 14 (genesis or the upgrade to 14), and a
//! document create against a contract that is not in state fails in the transformer, before this
//! loop, so no batch of an earlier protocol version reaches it: the shipped
//! `validate_state_v0` it hooks into behaves as it did for every such batch.

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::common::seated_moderation_charter::count_added_moderators;
use crate::execution::validation::state_transition::state_transitions::batch::state::v0::seated_charter_reads::{
    SeatedCharterRead, SeatedCharterReads,
};
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::contract_moderation::ModerationCharterAddedModeratorLimitReachedError;
use dpp::identifier::Identifier;
use dpp::moderation_charter::{
    property_names, ADDED_MODERATOR_DOCUMENT_TYPE_NAME, MODERATION_CHARTERS_CONTRACT_ID,
};
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{
    DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0,
};
use std::collections::BTreeMap;

/// The additions each seated charter was given by the creates of one batch accepted so far. One
/// per batch state validation.
#[derive(Default)]
pub(super) struct AddedModeratorCap {
    accepted_in_batch: BTreeMap<Identifier, u16>,
}

impl AddedModeratorCap {
    /// Refuses `create_action` when it is an `addedModerator` of the moderation charters
    /// contract for a charter that already has as many additions as its target allows, and
    /// counts it otherwise. Call it only for a create state validation accepted. A no-op, with
    /// nothing read, for every other create.
    ///
    /// The reads are billed: the elected charter by id and its target contract (once per batch,
    /// shared with the settle the addition forces), and the additions of the charter, at most
    /// the cap of them.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn validate_and_record_create(
        &mut self,
        create_action: &DocumentCreateTransitionAction,
        seated_charters: &mut SeatedCharterReads,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let base = create_action.base();
        if base.data_contract_id() != MODERATION_CHARTERS_CONTRACT_ID
            || base.document_type_name() != ADDED_MODERATOR_DOCUMENT_TYPE_NAME
        {
            return Ok(SimpleConsensusValidationResult::new());
        }
        let epoch = &block_info.epoch;
        // The schema requires the charter, and the create's reference validation proved it.
        let elected_charter_id = create_action
            .data()
            .get_identifier(property_names::ELECTED_CHARTER_ID)
            .map_err(|_| {
                Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "an addedModerator that passed state validation names its elected charter",
                ))
            })?;

        let SeatedCharterRead {
            charter,
            max_added_moderators,
        } = seated_charters.read(
            elected_charter_id,
            platform,
            epoch,
            execution_context,
            transaction,
            platform_version,
        )?;
        let target_contract_id = charter.charter.target_contract_id;
        let max_added_moderators = *max_added_moderators;

        let accepted_in_batch = self
            .accepted_in_batch
            .get(&elected_charter_id)
            .copied()
            .unwrap_or_default();
        let room_in_state = max_added_moderators.saturating_sub(accepted_in_batch);
        let in_state = count_added_moderators(
            platform.drive,
            elected_charter_id,
            room_in_state,
            epoch,
            execution_context,
            transaction,
            platform_version,
        )?;
        if in_state.saturating_add(accepted_in_batch) >= max_added_moderators {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ModerationCharterAddedModeratorLimitReachedError::new(
                    elected_charter_id,
                    target_contract_id,
                    max_added_moderators,
                )
                .into(),
            ));
        }
        self.accepted_in_batch
            .insert(elected_charter_id, accepted_in_batch.saturating_add(1));
        Ok(SimpleConsensusValidationResult::new())
    }
}
