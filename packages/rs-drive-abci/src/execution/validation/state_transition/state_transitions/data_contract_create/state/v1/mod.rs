use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::contract_group::{
    ContractGroupAlreadyExistsError, ContractGroupNotFoundError, IdentityNotContractGroupOwnerError,
};
use dpp::consensus::ConsensusError;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV1;
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

        let reference_result = {
            let StateTransitionAction::DataContractCreateAction(create_action) =
                action.data_as_borrowed()?
            else {
                return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "a valid data contract create state validation must contain a create action",
                )));
            };

            validate_data_contract_references(
                create_action.data_contract_ref(),
                platform.drive,
                block_info,
                execution_context,
                tx,
                platform_version,
            )?
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
        // and count the creating identity among its owners. Each lookup is billed. The signer
        // is authenticated by now, so a failure bumps the identity nonce and is paid.
        let contract_group_errors = validate_contract_groups_against_state(
            self,
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

/// Checks the transition's contract groups against the state: a registered group must be new,
/// and every group joined must exist and count the creating identity among its owners. Each
/// lookup is billed on the execution context.
fn validate_contract_groups_against_state<C: CoreRPCLike>(
    transition: &DataContractCreateTransition,
    platform: &PlatformRef<C>,
    block_info: &BlockInfo,
    tx: TransactionArg,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<Vec<ConsensusError>, Error> {
    {
        let registered_group_id = transition.contract_group_id();

        if let Some(contract_group_id) = registered_group_id {
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
        }

        let owner_id = transition.owner_id();
        let mut checked_groups = BTreeSet::new();
        for membership in transition.contract_group_memberships() {
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
                Some(info) if !info.owner().includes(&owner_id) => {
                    return Ok(vec![IdentityNotContractGroupOwnerError::new(
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
