//! The cap on the members the leader of a seated moderation team adds after the election.
//!
//! An `addedModerator` of the moderation charters system contract names a seated
//! `electedCharter` and a member the leader adds from the proposal's join requests. The target
//! contract's elected declaration caps them: at most `maxAddedModerators` per charter, counting
//! the additions ever filed (the type is immutable and undeletable), so a removal frees no slot.
//! The schema can not count documents, so this is a consensus rule of the document create,
//! judged here once the create's own state validation passed: its references then proved the
//! charter is seated, that its leader is the writer, and that the member asked to join.
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
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::common::seated_moderation_charter::count_added_moderators;
use crate::execution::validation::state_transition::state_transitions::batch::fetch_document_with_id;
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::contract_moderation::ModerationCharterAddedModeratorLimitReachedError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::document::DocumentV0Getters;
use dpp::identifier::Identifier;
use dpp::moderation_charter::{
    property_names, ADDED_MODERATOR_DOCUMENT_TYPE_NAME, ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
    MODERATION_CHARTERS_CONTRACT_ID,
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
    /// The reads are billed: the elected charter by id, its target contract, and the additions
    /// of the charter, at most the cap of them.
    pub(super) fn validate_and_record_create(
        &mut self,
        create_action: &DocumentCreateTransitionAction,
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

        let charters_contract = &base.data_contract_fetch_info_ref().contract;
        let elected_charter = fetch_document_with_id(
            platform.drive,
            charters_contract,
            charters_contract.document_type_for_name(ELECTED_CHARTER_DOCUMENT_TYPE_NAME)?,
            elected_charter_id,
            epoch,
            execution_context,
            transaction,
            platform_version,
        )?
        .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
            "the elected charter an addedModerator refers to was found by its reference",
        )))?;
        let target_contract_id = elected_charter
            .properties()
            .get_identifier(property_names::TARGET_CONTRACT_ID)
            .map_err(|_| {
                Error::Execution(ExecutionError::DriveIncoherence(
                    "a stored elected charter names its target contract",
                ))
            })?;

        // The fee this call returns is billed, never the one a cached fetch info carries,
        // which depends on the cache.
        let (fee, target_contract) = platform.drive.get_contract_with_fetch_info_and_fee(
            target_contract_id.to_buffer(),
            Some(epoch),
            false,
            transaction,
            platform_version,
        )?;
        let fee = fee.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
            "fee must exist when fetching a contract with an epoch",
        )))?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
        // A charter is only filed for a contract that declares elected moderation (its
        // `electionOpen` requirement), which is fixed at the contract's creation, and a contract
        // is never deleted.
        let max_added_moderators = target_contract
            .as_ref()
            .and_then(|fetch_info| {
                fetch_info
                    .contract
                    .config()
                    .moderation()
                    .and_then(|moderation| moderation.moderators.elected())
                    .map(|elected| elected.max_added_moderators)
            })
            .ok_or(Error::Execution(ExecutionError::DriveIncoherence(
                "the target of a stored elected charter declares elected moderation",
            )))?;

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
