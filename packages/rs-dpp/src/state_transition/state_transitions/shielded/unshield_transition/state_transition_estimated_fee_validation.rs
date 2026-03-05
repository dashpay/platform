use crate::fee::Credits;
use crate::state_transition::unshield_transition::UnshieldTransition;
use crate::state_transition::StateTransitionEstimatedFeeValidation;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl StateTransitionEstimatedFeeValidation for UnshieldTransition {
    fn calculate_min_required_fee(
        &self,
        _platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        // TODO: revisit when drive/drive-abci integration lands — the shielded fee is
        // embedded in unshielding_amount and validated on-chain via
        // `validate_minimum_shielded_fee`, but this client-side estimate currently
        // returns 0 which means mempool pre-checks won't reject under-fee'd txns.
        Ok(0)
    }
}
