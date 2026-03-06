use crate::fee::Credits;
use crate::state_transition::shield_transition::ShieldTransition;
use crate::state_transition::{
    StateTransitionEstimatedFeeValidation, StateTransitionWitnessSigned,
};
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl StateTransitionEstimatedFeeValidation for ShieldTransition {
    fn calculate_min_required_fee(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        let min_fees = &platform_version.fee_version.state_transition_min_fees;
        let input_count = self.inputs().len();
        Ok(min_fees
            .address_funds_transfer_input_cost
            .saturating_mul(input_count as u64))
    }
}
