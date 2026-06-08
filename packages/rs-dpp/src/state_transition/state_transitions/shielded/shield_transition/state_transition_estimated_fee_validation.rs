use crate::fee::Credits;
use crate::shielded::compute_shielded_verification_fee;
use crate::state_transition::shield_transition::ShieldTransition;
use crate::state_transition::StateTransitionEstimatedFeeValidation;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl StateTransitionEstimatedFeeValidation for ShieldTransition {
    /// Returns an **advisory** lower bound on a transparent `Shield`'s fee: the predictable
    /// COMPUTE portion `compute_shielded_verification_fee(num_actions)` (the per-bundle Halo 2
    /// proof-verification fee + the per-action processing fee).
    ///
    /// This is NOT the funding floor and is NOT the full fee. Unlike the asset-lock transitions,
    /// the transparent `Shield` is excluded from the address-minimum-balance pre-check
    /// (`StateTransition::required_asset_lock_balance_for_processing_start` does not dispatch to it),
    /// so no consensus or funding path consumes this value — it is only reachable via the generic
    /// SDK advisory `StateTransition::calculate_estimated_fee()`.
    ///
    /// `Shield` is metered + compute: GroveDB meters the real storage and processing of the
    /// note/nullifier writes, and the execution-event layer adds only this compute fee on top (see
    /// `validate_fees_of_event` / the `ExecutionEvent` construction). The metered storage/processing
    /// portion depends on live GroveDB tree state and therefore cannot be known statelessly here, so
    /// the real fee a `Shield` will be charged is strictly greater than this number. A caller that
    /// needs the actual required balance must price the metered portion against state (e.g. via the
    /// fee-validation pass), not rely on this estimate.
    fn calculate_min_required_fee(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        // The on-wire Orchard `actions` count is what the compute fee is priced against (matching
        // the consensus structure-validation floor and the execution-event compute charge).
        let ShieldTransition::V0(v0) = self;
        compute_shielded_verification_fee(v0.actions.len(), platform_version)
    }
}
