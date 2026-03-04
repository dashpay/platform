use crate::fee::Credits;
use crate::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
use crate::state_transition::StateTransitionEstimatedFeeValidation;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl StateTransitionEstimatedFeeValidation for ShieldedWithdrawalTransition {
    fn calculate_min_required_fee(
        &self,
        _platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        // Fee for shielded withdrawal is paid from value balance in the orchard bundle
        // Minimum fee is 0 as the actual fee is extracted from the bundle during validation
        Ok(0)
    }
}
