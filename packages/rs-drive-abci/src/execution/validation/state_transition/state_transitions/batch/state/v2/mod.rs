use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{PartialIdentity, Purpose};
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::token_transition::TokenTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::{StateTransitionIdentitySigned, StateTransitionOwned};
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::{
    GasPayer, ResolvedContractGroupMemberships, ResolvedGasSponsor,
};
use drive::state_transition_action::StateTransitionAction;
use std::collections::BTreeSet;

use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::state_transitions::batch::transformer::v0::BatchTransitionTransformerV0;
use crate::platform_types::platform::PlatformStateRef;
use crate::platform_types::platform_state::PlatformStateV0Methods;

/// PROTOCOL_VERSION_14+: like v1, and when the batch is signed by an AUTHENTICATION key bound to
/// a contract group, the action also carries the contract group memberships of every contract
/// the batch touches.
///
/// Whether a member lies inside a group bound is a question about state: does the member's
/// contract, its document type or its token belong to the group? The action is the state-based
/// translation of the transition, so the answer's raw material is resolved here, where state is
/// read, and advanced structure validation judges the key's bounds from the action without
/// touching Drive.
///
/// The signer's bounds are resolved first, from the identity the signature was validated
/// against, and only a group-bound key triggers the read: a batch signed by any other key does
/// exactly what it did under v1, with no extra read and nothing new that can fail. The read is
/// not billed here either. Its fee travels with the memberships (as a contract's fetch info
/// carries its own) and is billed by the check that uses them.
pub(in crate::execution::validation::state_transition::state_transitions::batch) trait DocumentsBatchStateTransitionStateValidationV2
{
    fn transform_into_action_v2(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        signer_identity: Option<&PartialIdentity>,
        validate_against_state: bool,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DocumentsBatchStateTransitionStateValidationV2 for BatchTransition {
    fn transform_into_action_v2(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        signer_identity: Option<&PartialIdentity>,
        validate_against_state: bool,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let signed_by_a_group_bound_key = signer_identity
            .and_then(|identity| {
                identity
                    .loaded_public_keys
                    .get(&self.signature_public_key_id())
            })
            .filter(|key| key.purpose() == Purpose::AUTHENTICATION)
            .and_then(|key| key.contract_bounds())
            .and_then(|bounds| bounds.contract_group_id())
            .is_some();

        let mut validation_result = self.try_into_action_v0(
            platform,
            block_info,
            validate_against_state,
            tx,
            execution_context,
        )?;

        // A result with errors never reaches the bounds check, so nothing is resolved for it.
        let bounds_will_be_checked =
            signed_by_a_group_bound_key && validation_result.errors.is_empty();
        if let Some(action) = validation_result
            .data
            .as_mut()
            .filter(|_| bounds_will_be_checked)
        {
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

        // A batch whose every transition asks the contract owner to pay its gas, on a document
        // type that offers it, names that contract owner its sponsor. Their balance is read here,
        // billed to the batch, and judged by fee validation against the fee. The batch's own
        // signer is never their own sponsor. A batch that already has errors, or asks for a payer
        // the contracts do not offer, resolves nothing: advanced structure validation raises the
        // latter, and the signer pays for the former.
        if let Some(action) = validation_result
            .data
            .as_mut()
            .filter(|_| validation_result.errors.is_empty())
        {
            if let Ok(GasPayer::ContractOwner {
                identity_id,
                strict,
            }) = action.resolve_gas_payer()
            {
                if identity_id != self.owner_id() {
                    let platform_version = platform.state.current_platform_version()?;
                    let (balance, fee) = platform.drive.fetch_identity_balance_with_costs(
                        identity_id.to_buffer(),
                        block_info,
                        true,
                        tx,
                        platform_version,
                    )?;
                    execution_context
                        .add_operation(ValidationOperation::PrecalculatedOperation(fee));
                    action.set_gas_sponsor(Some(ResolvedGasSponsor {
                        identity_id,
                        balance: balance.unwrap_or_default(),
                        strict,
                    }));
                }
            }
        }

        Ok(validation_result.map(Into::into))
    }
}
