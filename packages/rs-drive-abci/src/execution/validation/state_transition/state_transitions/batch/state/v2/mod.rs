use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::token_transition::TokenTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::batch_transition::BatchTransition;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::ResolvedContractGroupMemberships;
use drive::state_transition_action::StateTransitionAction;
use std::collections::BTreeSet;

use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::state_transitions::batch::transformer::v0::BatchTransitionTransformerV0;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformStateRef;
use crate::platform_types::platform_state::PlatformStateV0Methods;

/// PROTOCOL_VERSION_14+: like v1, and the action also carries the contract group memberships of
/// every contract the batch touches.
///
/// A batch may be signed by an AUTHENTICATION key bound to a contract group, and whether a
/// member lies inside that bound is a question about state: does the member's contract, its
/// document type or its token belong to the group? The action is the state-based translation
/// of the transition, so the answer's raw material is resolved here, where state is read, and
/// advanced structure validation judges the key's bounds from the action without touching
/// Drive.
///
/// The read is not billed here. Its fee travels with the memberships (as a contract's fetch
/// info carries its own) and is billed by the check that uses them, so a batch signed by an
/// ordinary key costs what it cost under v1. For a contract in no group the read is one miss
/// on the contract's entry of the backwards index.
pub(in crate::execution::validation::state_transition::state_transitions::batch) trait DocumentsBatchStateTransitionStateValidationV2
{
    fn transform_into_action_v2(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DocumentsBatchStateTransitionStateValidationV2 for BatchTransition {
    fn transform_into_action_v2(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let mut validation_result = self.try_into_action_v0(
            platform,
            block_info,
            validation_mode.should_validate_batch_valid_against_state(),
            tx,
            execution_context,
        )?;

        if let Some(action) = validation_result.data.as_mut() {
            let platform_version = platform.state.current_platform_version()?;
            let contract_ids: BTreeSet<Identifier> = self
                .transitions_iter()
                .map(|member| match member {
                    BatchedTransitionRef::Document(document) => document.data_contract_id(),
                    BatchedTransitionRef::Token(token) => token.data_contract_id(),
                })
                .collect();
            for contract_id in contract_ids {
                let (fee, memberships) = platform
                    .drive
                    .fetch_contract_group_memberships_for_contract_with_fee(
                        contract_id,
                        &block_info.epoch,
                        tx,
                        platform_version,
                    )?;
                action.set_contract_group_memberships(
                    contract_id,
                    ResolvedContractGroupMemberships { memberships, fee },
                );
            }
        }

        Ok(validation_result.map(Into::into))
    }
}
