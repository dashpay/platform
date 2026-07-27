use crate::fee::Credits;
use crate::state_transition::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
use crate::state_transition::StateTransitionEstimatedFeeValidation;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl StateTransitionEstimatedFeeValidation for IdentityCreateFromShieldedPoolTransition {
    fn calculate_min_required_fee(
        &self,
        _platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        // Like the other pool-spend shielded transitions, the fee is carved from the exit
        // denomination and validated on-chain (the denomination must cover the metered + compute
        // fee). The client-side predictor lives in `compute_shielded_identity_create_fee`; this
        // mempool pre-check estimate returns 0 (mirroring `Unshield`).
        Ok(0)
    }
}
