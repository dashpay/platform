use crate::error::Error;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValueGettersV0;
use dpp::consensus::state::address_funds::AddressesNotEnoughFundsError;
use dpp::fee::Credits;
use dpp::state_transition::address_funding_from_asset_lock_transition::accessors::AddressFundingFromAssetLockTransitionAccessorsV0;
use dpp::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::state_transition_action::address_funds::address_funding_from_asset_lock::AddressFundingFromAssetLockTransitionAction;
use drive::state_transition_action::system::partially_use_asset_lock_action::PartiallyUseAssetLockAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::address_funding_from_asset_lock) trait AddressFundingFromAssetLockStateTransitionAdvancedStructureValidationV0
{
    fn validate_advanced_structure_from_state_v0(
        &self,
        action: AddressFundingFromAssetLockTransitionAction,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl AddressFundingFromAssetLockStateTransitionAdvancedStructureValidationV0
    for AddressFundingFromAssetLockTransition
{
    fn validate_advanced_structure_from_state_v0(
        &self,
        mut action: AddressFundingFromAssetLockTransitionAction,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        // Calculate total available funds (asset lock + input amounts from transition)
        let asset_lock_remaining = action
            .asset_lock_value_to_be_consumed()
            .remaining_credit_value();

        // Use the transition's input amounts (what's being spent), not the remaining balances
        let inputs_total: Credits = self.inputs().values().map(|(_, amount)| *amount).sum();
        let total_available = asset_lock_remaining.saturating_add(inputs_total);

        // Calculate sum of explicit outputs (Some values only)
        let explicit_outputs_sum: Credits = self.outputs().values().filter_map(|v| *v).sum();

        // Validate that we have enough funds for explicit outputs
        if total_available < explicit_outputs_sum {
            // Apply penalty for insufficient funds
            let penalty = platform_version
                .drive_abci
                .validation_and_processing
                .penalties
                .address_funds_insufficient_balance;

            let used_credits = penalty
                .checked_add(execution_context.fee_cost(platform_version)?.processing_fee)
                .ok_or(ProtocolError::Overflow("processing fee overflow error"))?;

            // Create a PartiallyUseAssetLockAction to deduct fees from inputs first, then asset lock
            let bump_action = StateTransitionAction::PartiallyUseAssetLockAction(
                PartiallyUseAssetLockAction::from_address_funding_from_asset_lock_transition_action(
                    action,
                    used_credits,
                ),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                vec![AddressesNotEnoughFundsError::new(
                    self.inputs().clone(),
                    explicit_outputs_sum,
                )
                .into()],
            ));
        }

        // Determine if remainder output should be removed
        // If total_available == explicit_outputs_sum, there's nothing left for remainder
        let should_remove_remainder = total_available == explicit_outputs_sum;

        if should_remove_remainder {
            // Remove the None output (remainder) from the action
            action.remove_remainder_output();
        }

        Ok(ConsensusValidationResult::new_with_data(action.into()))
    }
}
