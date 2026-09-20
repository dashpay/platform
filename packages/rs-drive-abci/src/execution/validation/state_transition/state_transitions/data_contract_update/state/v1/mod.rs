use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::contract_moderation::ContractModeratorIdentityNotFoundError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::common::validate_identity_exists::validate_identity_exists;
use crate::execution::validation::state_transition::data_contract_common::data_contract_reference_validation::validate_data_contract_references;
use crate::execution::validation::state_transition::state_transitions::data_contract_update::state::v0::DataContractUpdateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) trait DataContractUpdateStateTransitionStateValidationV1
{
    fn validate_state_v1<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DataContractUpdateStateTransitionStateValidationV1 for DataContractUpdateTransition {
    fn validate_state_v1<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let action = self.validate_state_v0::<C>(
            platform,
            block_info,
            validation_mode,
            execution_context,
            tx,
            platform_version,
        )?;

        if !action.is_valid() {
            return Ok(action);
        }

        // The updated contract may add document types or properties carrying
        // reference declarations, so they are re-validated on every update
        let (reference_result, contract_id, added_moderators) = {
            let StateTransitionAction::DataContractUpdateAction(update_action) =
                action.data_as_borrowed()?
            else {
                return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "a valid data contract update state validation must contain an update action",
                )));
            };

            let contract = update_action.data_contract_ref();

            (
                validate_data_contract_references(
                    contract,
                    platform.drive,
                    block_info,
                    execution_context,
                    tx,
                    platform_version,
                )?,
                contract.id(),
                moderators_added_by_the_update(
                    contract,
                    platform,
                    block_info,
                    execution_context,
                    tx,
                    platform_version,
                )?,
            )
        };

        if !reference_result.is_valid() {
            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                StateTransitionAction::BumpIdentityDataContractNonceAction(
                    BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                        self,
                    ),
                ),
                reference_result.errors,
            ));
        }

        // Contract moderation: an identity the update names as a moderator must exist. One
        // that does not can never sign a moderation, so naming it is a mistake, caught here
        // once rather than in every feature that will read the set. Each lookup is billed; a
        // miss is paid like the one above.
        for moderator_id in &added_moderators {
            if !validate_identity_exists(
                platform.drive,
                moderator_id,
                execution_context,
                tx,
                platform_version,
            )? {
                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                    StateTransitionAction::BumpIdentityDataContractNonceAction(
                        BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                            self,
                        ),
                    ),
                    vec![ContractModeratorIdentityNotFoundError::new(contract_id, *moderator_id)
                        .into()],
                ));
            }
        }

        Ok(action)
    }
}

/// The moderator identities `contract` names that the stored contract does not, the owner
/// left out (it signed this transition, so it exists). The ones the stored contract names
/// were checked when they were added, and identities are never removed, so they are not
/// looked up again. The read of the stored contract is billed like every other read here, from
/// the fee the fetch returns (never from the fee a cached fetch info carries, which depends on
/// the node's cache), whether it was served from the cache or from disk.
fn moderators_added_by_the_update<C: CoreRPCLike>(
    contract: &DataContract,
    platform: &PlatformRef<C>,
    block_info: &BlockInfo,
    execution_context: &mut StateTransitionExecutionContext,
    tx: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Vec<Identifier>, Error> {
    let Some(named) = contract
        .config()
        .moderation()
        .and_then(|moderation| moderation.moderators.identity_ids())
    else {
        return Ok(vec![]);
    };

    let (fee, stored) = platform.drive.get_contract_with_fetch_info_and_fee(
        contract.id().to_buffer(),
        Some(&block_info.epoch),
        false,
        tx,
        platform_version,
    )?;
    if let Some(fee) = fee {
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
    }
    let already_named = |identity_id: &Identifier| {
        stored.as_ref().is_some_and(|stored| {
            stored
                .contract
                .config()
                .moderation()
                .is_some_and(|moderation| moderation.moderators.names(identity_id))
        })
    };

    let owner_id = contract.owner_id();
    Ok(named
        .iter()
        .filter(|id| **id != owner_id && !already_named(id))
        .copied()
        .collect())
}
