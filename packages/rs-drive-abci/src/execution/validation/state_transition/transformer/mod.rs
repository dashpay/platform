use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::StateTransition;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

/// A trait for validating state transitions within a blockchain.
pub trait StateTransitionActionTransformerV0 {
    /// Transforms a `TransactionArg` into a `StateTransitionAction`, primarily for testing purposes.
    ///
    /// This function should not be called directly in production since the functionality is already contained within `validate_state`.
    ///
    /// Explanation why the structure isn't versioned: if for some reason we need to change the form of transform_into_action
    /// It should be done by creating a new trait
    ///
    /// # Type Parameters
    /// * `C`: A type implementing the `CoreRPCLike` trait.
    ///
    /// # Arguments
    /// * `platform`: A reference to a platform implementing CoreRPCLike.
    /// * `tx`: The `TransactionArg` to be transformed into a `StateTransitionAction`.
    ///
    /// # Returns
    /// A `Result` containing either a `ConsensusValidationResult<StateTransitionAction>` or an `Error`.
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionActionTransformerV0 for StateTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match self {
            StateTransition::DataContractCreate(st) => st.transform_into_action(
                platform,
                block_info,
                validation_mode,
                execution_context,
                tx,
            ),
            StateTransition::DataContractUpdate(st) => st.transform_into_action(
                platform,
                block_info,
                validation_mode,
                execution_context,
                tx,
            ),
            StateTransition::IdentityCreate(st) => st.transform_into_action(
                platform,
                block_info,
                validation_mode,
                execution_context,
                tx,
            ),
            StateTransition::IdentityUpdate(st) => st.transform_into_action(
                platform,
                block_info,
                validation_mode,
                execution_context,
                tx,
            ),
            StateTransition::IdentityTopUp(st) => st.transform_into_action(
                platform,
                block_info,
                validation_mode,
                execution_context,
                tx,
            ),
            StateTransition::IdentityCreditWithdrawal(st) => st.transform_into_action(
                platform,
                block_info,
                validation_mode,
                execution_context,
                tx,
            ),
            StateTransition::DocumentsBatch(st) => st.transform_into_action(
                platform,
                block_info,
                validation_mode,
                execution_context,
                tx,
            ),
            StateTransition::IdentityCreditTransfer(st) => st.transform_into_action(
                platform,
                block_info,
                validation_mode,
                execution_context,
                tx,
            ),
        }
    }
}
