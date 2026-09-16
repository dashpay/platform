use crate::fee::Credits;
use crate::state_transition::token_unshield_with_shielded_fee_transition::TokenUnshieldWithShieldedFeeTransition;
use crate::state_transition::StateTransitionEstimatedFeeValidation;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl StateTransitionEstimatedFeeValidation for TokenUnshieldWithShieldedFeeTransition {
    /// Pool-paid: the fee is carved from the fee bundle's value balance and enforced by the
    /// shielded minimum-fee validation, not by an identity balance floor.
    fn calculate_min_required_fee(
        &self,
        _platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        Ok(0)
    }
}
