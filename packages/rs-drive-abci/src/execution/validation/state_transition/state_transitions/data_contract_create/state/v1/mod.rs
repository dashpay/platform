use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::contract_group::{
    ContractGroupMembershipsOverLimitError, DuplicateContractGroupMembershipError,
    RedundantContractGroupMembershipError,
};
use dpp::consensus::state::contract_group::{
    ContractGroupAdminNotFoundError, ContractGroupAlreadyExistsError, ContractGroupNotFoundError,
    IdentityNotContractGroupOwnerOrAdminError,
};
use dpp::consensus::ConsensusError;
use dpp::contract_group::ContractGroupMember;
use dpp::identifier::Identifier;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::data_contract_create_transition::accessors::{
    DataContractCreateTransitionAccessorsV0, DataContractCreateTransitionAccessorsV1,
};
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::state_transition::StateTransitionOwned;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceAction;
use drive::state_transition_action::StateTransitionAction;
use std::collections::BTreeSet;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::common::validate_non_masternode_identity_exists::validate_non_masternode_identity_exists;
use crate::execution::validation::state_transition::data_contract_common::data_contract_reference_validation::validate_data_contract_references;
use crate::execution::validation::state_transition::state_transitions::data_contract_create::state::v0::DataContractCreateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_create) trait DataContractCreateStateTransitionStateValidationV1
{
    fn validate_state_v1<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DataContractCreateStateTransitionStateValidationV1 for DataContractCreateTransition {
    fn validate_state_v1<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let action = self.validate_state_v0::<C>(
            platform,
            block_info,
            validation_mode,
            tx,
            execution_context,
            platform_version,
        )?;

        if !action.is_valid() {
            return Ok(action);
        }

        let (reference_result, registered_group_id) = {
            let StateTransitionAction::DataContractCreateAction(create_action) =
                action.data_as_borrowed()?
            else {
                return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "a valid data contract create state validation must contain a create action",
                )));
            };

            // The action already carries the derived group id, so it is not hashed again here.
            let registered_group_id = create_action
                .contract_group()
                .map(|(contract_group_id, _)| *contract_group_id);

            (
                validate_data_contract_references(
                    create_action.data_contract_ref(),
                    platform.drive,
                    block_info,
                    execution_context,
                    tx,
                    platform_version,
                )?,
                registered_group_id,
            )
        };

        if !reference_result.is_valid() {
            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                StateTransitionAction::BumpIdentityNonceAction(
                    BumpIdentityNonceAction::from_borrowed_data_contract_create_transition(self),
                ),
                reference_result.errors,
            ));
        }

        // Contract groups: the registered group must be new, and every group joined must exist
        // and have the creating identity as its owner or an admin. Each lookup is billed. The signer
        // is authenticated by now, so a failure bumps the identity nonce and is paid.
        let contract_group_errors = validate_contract_groups_against_state(
            self,
            registered_group_id,
            platform,
            block_info,
            tx,
            execution_context,
            platform_version,
        )?;
        if !contract_group_errors.is_empty() {
            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                StateTransitionAction::BumpIdentityNonceAction(
                    BumpIdentityNonceAction::from_borrowed_data_contract_create_transition(self),
                ),
                contract_group_errors,
            ));
        }

        Ok(action)
    }
}

/// Checks the transition's contract groups against the state: a registered group must be new
/// and its admins must be existing non-masternode identities, and every group joined must exist
/// and have the creating identity as its owner or an admin. Each lookup is billed on the execution
/// context, as is the group id derivation the action performed.
fn validate_contract_groups_against_state<C: CoreRPCLike>(
    transition: &DataContractCreateTransition,
    registered_group_id: Option<Identifier>,
    platform: &PlatformRef<C>,
    block_info: &BlockInfo,
    tx: TransactionArg,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<Vec<ConsensusError>, Error> {
    {
        if let Some(contract_group_id) = registered_group_id {
            // The group id is a double SHA-256 over one block, computed when the action was
            // built; bill it like the contract id derivation is.
            execution_context.add_operation(ValidationOperation::DoubleSha256(1));

            let (fee, existing) = platform.drive.fetch_contract_group_info_with_fee(
                contract_group_id,
                &block_info.epoch,
                tx,
                platform_version,
            )?;
            execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
            if existing.is_some() {
                return Ok(vec![ContractGroupAlreadyExistsError::new(
                    contract_group_id,
                )
                .into()]);
            }

            // Every admin must be an existing non-masternode identity, as change-control group
            // members must be: memberships are creation-only, so a mistyped admin could never be
            // replaced. Each lookup is billed by the helper.
            if let Some(registration) = transition.contract_group() {
                for admin_id in &registration.admins {
                    let admin_exists = validate_non_masternode_identity_exists(
                        platform.drive,
                        admin_id,
                        execution_context,
                        tx,
                        platform_version,
                    )?;
                    if !admin_exists {
                        return Ok(vec![ContractGroupAdminNotFoundError::new(
                            contract_group_id,
                            *admin_id,
                        )
                        .into()]);
                    }
                }
            }
        }

        let memberships = transition.contract_group_memberships();
        if !memberships.is_empty() {
            // The per-contract cap, duplicate and redundancy rules must hold against what the
            // contract already holds, not only within this transition: today a contract
            // declares memberships once, at creation, but the rules survive a later join path
            // only if the state is consulted. One billed read of Members/<contract id>.
            let (fee, existing) = platform
                .drive
                .fetch_contract_group_memberships_for_contract_with_fee(
                    transition.data_contract().id(),
                    &block_info.epoch,
                    tx,
                    platform_version,
                )?;
            execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
            let cap = platform_version
                .system_limits
                .max_contract_group_memberships_per_contract as usize;
            if existing.len() + memberships.len() > cap {
                return Ok(vec![ContractGroupMembershipsOverLimitError::new(
                    (existing.len() + memberships.len()) as u32,
                    platform_version
                        .system_limits
                        .max_contract_group_memberships_per_contract,
                )
                .into()]);
            }
            for membership in memberships {
                if existing.contains(membership) {
                    return Ok(vec![DuplicateContractGroupMembershipError::new(
                        membership.contract_group_id,
                        membership.member.clone(),
                    )
                    .into()]);
                }
                let redundant = match &membership.member {
                    ContractGroupMember::Contract => existing
                        .all_contract_group_ids()
                        .contains(&membership.contract_group_id),
                    _ => existing.contract.contains(&membership.contract_group_id),
                };
                if redundant {
                    return Ok(vec![RedundantContractGroupMembershipError::new(
                        membership.contract_group_id,
                        membership.member.clone(),
                    )
                    .into()]);
                }
            }
        }

        let owner_id = transition.owner_id();
        let mut checked_groups = BTreeSet::new();
        for membership in memberships {
            let contract_group_id = membership.contract_group_id;
            // The group registered by this same transition is owned by the signer by construction.
            if Some(contract_group_id) == registered_group_id
                || !checked_groups.insert(contract_group_id)
            {
                continue;
            }
            let (fee, info) = platform.drive.fetch_contract_group_info_with_fee(
                contract_group_id,
                &block_info.epoch,
                tx,
                platform_version,
            )?;
            execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
            match info {
                None => {
                    return Ok(vec![
                        ContractGroupNotFoundError::new(contract_group_id).into()
                    ]);
                }
                Some(info) if !info.owner().may_add_members(&owner_id) => {
                    return Ok(vec![IdentityNotContractGroupOwnerOrAdminError::new(
                        owner_id,
                        contract_group_id,
                    )
                    .into()]);
                }
                Some(_) => {}
            }
        }

        Ok(vec![])
    }
}
