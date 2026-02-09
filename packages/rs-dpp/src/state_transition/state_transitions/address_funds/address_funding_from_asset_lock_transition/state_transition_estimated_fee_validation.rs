use crate::balances::credits::CREDITS_PER_DUFF;
use crate::fee::Credits;
use crate::state_transition::address_funding_from_asset_lock_transition::accessors::AddressFundingFromAssetLockTransitionAccessorsV0;
use crate::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
use crate::state_transition::{
    StateTransitionEstimatedFeeValidation, StateTransitionWitnessSigned,
};
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl StateTransitionEstimatedFeeValidation for AddressFundingFromAssetLockTransition {
    fn calculate_min_required_fee(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        let min_fees = &platform_version.fee_version.state_transition_min_fees;
        let asset_lock_base_cost = platform_version
            .dpp
            .state_transitions
            .identities
            .asset_locks
            .required_asset_lock_duff_balance_for_processing_start_for_address_funding
            * CREDITS_PER_DUFF;
        let input_count = self.inputs().len();
        let output_count = self.outputs().len().max(1);
        Ok(asset_lock_base_cost.saturating_add(
            min_fees
                .address_funds_transfer_input_cost
                .saturating_mul(input_count as u64)
                .saturating_add(
                    min_fees
                        .address_funds_transfer_output_cost
                        .saturating_mul(output_count as u64),
                ),
        ))
    }
}
