use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::state_transitions::batch::transformer::v0::BatchTransitionInternalTransformerV0;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::contract_moderation::{
    ContractUserBannedError, ContractUserSuspendedError,
};
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::moderation::ContractModerationList;
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::batch_transition::batched_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use std::collections::{BTreeMap, BTreeSet};

pub(super) trait BatchTransitionContractModerationGateV0 {
    #[allow(clippy::too_many_arguments)]
    fn contract_moderation_gate_v0(
        drive: &Drive,
        block_info: &BlockInfo,
        contract: &DataContract,
        owner_id: Identifier,
        document_transitions: &BTreeMap<&String, Vec<&DocumentTransition>>,
        lapsed_suspensions: &mut BTreeSet<(Identifier, Identifier)>,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ConsensusValidationResult<Vec<BatchedTransitionAction>>>, Error>;
}

impl BatchTransitionContractModerationGateV0 for BatchTransition {
    /// A moderated contract refuses every document transition of an identity on its banlist
    /// or under a live suspension, and the first document transition after a suspension lapsed
    /// sweeps the stale entry. The gate runs in the transformer, so the mempool refuses a
    /// barred identity as a block does. A contract that declares no moderation costs nothing:
    /// no read is made for it.
    fn contract_moderation_gate_v0(
        drive: &Drive,
        block_info: &BlockInfo,
        contract: &DataContract,
        owner_id: Identifier,
        document_transitions: &BTreeMap<&String, Vec<&DocumentTransition>>,
        lapsed_suspensions: &mut BTreeSet<(Identifier, Identifier)>,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ConsensusValidationResult<Vec<BatchedTransitionAction>>>, Error> {
        let Some(moderation) = contract.config().moderation() else {
            return Ok(None);
        };
        let data_contract_id = contract.id();

        let lists: Vec<ContractModerationList> = moderation.lists().collect();
        let (fee, status) = drive.fetch_contract_moderation_status_with_fee(
            data_contract_id,
            owner_id,
            &lists,
            &block_info.epoch,
            transaction,
            platform_version,
        )?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));

        let barred: Option<ConsensusError> = if status.banned {
            Some(ContractUserBannedError::new(data_contract_id, owner_id).into())
        } else {
            status
                .suspended_until
                .filter(|until| *until > block_info.time_ms)
                .map(|until| {
                    ContractUserSuspendedError::new(data_contract_id, owner_id, until).into()
                })
        };

        let Some(error) = barred else {
            if status.has_lapsed_suspension_at(block_info.time_ms) {
                lapsed_suspensions.insert((data_contract_id, owner_id));
            }
            return Ok(None);
        };

        // Paid: the signer is authenticated and the read happened. Every transition against
        // the contract is refused on its own, as a per-transition failure is anywhere else in
        // the transformer, so each one's contract nonce is bumped and none stays replayable.
        let mut actions = vec![];
        let mut errors = vec![];
        for transition in document_transitions.values().flatten() {
            let failed = Self::failed_per_transition_action(
                transition.base(),
                owner_id,
                vec![error.clone()],
                platform_version,
            )?;
            actions.extend(failed.data);
            errors.extend(failed.errors);
        }

        Ok(Some(if actions.is_empty() {
            ConsensusValidationResult::new_with_errors(if errors.is_empty() {
                vec![error]
            } else {
                errors
            })
        } else {
            ConsensusValidationResult::new_with_data_and_errors(actions, errors)
        }))
    }
}
