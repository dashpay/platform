//! The settle of an elected contract's moderators pot that a change of its seated team forces
//! first.
//!
//! The team of a seated moderation charter changes with its leader's `addedModerator` and
//! `removedModerator` documents of the moderation charters contract, created or deleted (an
//! addition is taken back by deleting it, a removal of an elected member undone by deleting
//! it). Before any of them the target's moderators pot is paid out to the team as it was, by
//! its proposal's reward split and the action counts since the last settle, and the counts
//! start over: so no member loses what it earned with the team it earned it in, and no member
//! joins in on what was earned before it came. The once-per-epoch limit of a claim does not
//! apply, and the settle is not a claim: it leaves the pot's last claim alone, so the team may
//! still claim in the same epoch. A pot too small to pay anyone a credit is settled all the
//! same: the counts start over.
//!
//! The settle is an effect of the change, never a refusal: it is judged once the change's own
//! state validation (and for an addition the cap on additions) accepted it, and the batch
//! carries what it pays, for the batch converter to write before the change. Like the cap it
//! runs in state validation, which check tx does not run for a batch: the mempool admits the
//! change without reading the pot, and prices it without the settle's reads and writes. At most one settle per target contract per batch: a later
//! change of the same batch finds the pot paid out and the counts reset. That is dormant while
//! `max_transitions_in_documents_batch` is 1, as it is at every protocol version.
//!
//! Only a create or a delete of the charter contract's `addedModerator` or `removedModerator`
//! takes this path. The charter contract exists in state from protocol version 14 (genesis or
//! the upgrade to 14), and a document transition against a contract that is not in state fails
//! in the transformer, before the state validation loop, so no batch of an earlier protocol
//! version reaches it: the shipped `validate_state_v0` it hooks into behaves as it did for
//! every such batch.

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::state_transitions::batch::fetch_document_with_id;
use crate::execution::validation::state_transition::state_transitions::batch::state::v0::seated_charter_reads::{
    SeatedCharterRead, SeatedCharterReads,
};
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use dpp::document::DocumentV0Getters;
use dpp::identifier::Identifier;
use dpp::moderation_charter::{
    property_names, ADDED_MODERATOR_DOCUMENT_TYPE_NAME, MODERATION_CHARTERS_CONTRACT_ID,
    REMOVED_MODERATOR_DOCUMENT_TYPE_NAME,
};
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentCreateTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use drive::state_transition_action::contract::moderators_pot_settlement::ModeratorsPotSettlement;
use std::collections::BTreeSet;

/// The settles one batch forces. One per batch state validation.
#[derive(Default)]
pub(super) struct ModeratorsPotSettles {
    settled_targets: BTreeSet<Identifier>,
    settlements: Vec<ModeratorsPotSettlement>,
}

impl ModeratorsPotSettles {
    /// When `transition` changes a seated team (a create or a delete of the moderation
    /// charters contract's `addedModerator` or `removedModerator`), settles the target's
    /// moderators pot to the team as it is before the change. A no-op, with nothing read, for
    /// every other transition. Call it only for a transition state validation accepted.
    ///
    /// The reads are billed: for a delete the team change being deleted, the elected charter by
    /// id and its target contract (once per batch, shared with the cap on additions), the pot,
    /// and what the settle reads (the team, the proposal and the action counts).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn settle_before_team_change(
        &mut self,
        transition: &BatchedTransitionAction,
        seated_charters: &mut SeatedCharterReads,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let BatchedTransitionAction::DocumentAction(document_action) = transition else {
            return Ok(());
        };
        let base = document_action.base();
        let document_type_name = base.document_type_name();
        if base.data_contract_id() != MODERATION_CHARTERS_CONTRACT_ID
            || (document_type_name != ADDED_MODERATOR_DOCUMENT_TYPE_NAME
                && document_type_name != REMOVED_MODERATOR_DOCUMENT_TYPE_NAME)
        {
            return Ok(());
        }
        let epoch = &block_info.epoch;
        let charters_contract = &base.data_contract_fetch_info_ref().contract;
        let unnamed_charter = || {
            Error::Execution(ExecutionError::DriveIncoherence(
                "a moderation team change names its elected charter",
            ))
        };
        let elected_charter_id = match document_action {
            // The schema requires the charter, and the create's reference validation proved it.
            DocumentTransitionAction::CreateAction(create_action) => create_action
                .data()
                .get_identifier(property_names::ELECTED_CHARTER_ID)
                .map_err(|_| unnamed_charter())?,
            // The delete's state validation found the change and its owner, the leader.
            DocumentTransitionAction::DeleteAction(_) => fetch_document_with_id(
                platform.drive,
                charters_contract,
                charters_contract.document_type_for_name(document_type_name)?,
                base.id(),
                epoch,
                execution_context,
                transaction,
                platform_version,
            )?
            .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "a moderation team change the delete's state validation found is stored",
            )))?
            .properties()
            .get_identifier(property_names::ELECTED_CHARTER_ID)
            .map_err(|_| unnamed_charter())?,
            // The team change types are immutable, neither transferable nor tradeable, and have
            // no indexOnly storage: nothing else of them passes state validation.
            _ => return Ok(()),
        };

        // A team change can only name a stored elected charter, which is a seated one: only a
        // contest's winner is ever written to the type's storage. The cap on additions read it
        // already for an addition.
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
        if !self.settled_targets.insert(target_contract_id) {
            return Ok(());
        }

        let (fee, fee_pot) = platform.drive.fetch_contract_fee_pot_with_fee(
            target_contract_id,
            ContractFeePot::Moderators,
            epoch,
            transaction,
            platform_version,
        )?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));

        let settlement = charter.settle_moderators_pot(
            platform.drive,
            target_contract_id,
            fee_pot.credits,
            *max_added_moderators,
            epoch,
            execution_context,
            transaction,
            platform_version,
        )?;
        if !settlement.is_empty() {
            self.settlements.push(settlement);
        }
        Ok(())
    }

    /// What the batch pays out and resets before its changes
    pub(super) fn into_settlements(self) -> Vec<ModeratorsPotSettlement> {
        self.settlements
    }
}
