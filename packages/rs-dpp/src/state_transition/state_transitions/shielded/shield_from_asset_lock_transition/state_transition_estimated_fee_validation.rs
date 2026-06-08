use crate::balances::credits::CREDITS_PER_DUFF;
use crate::fee::Credits;
use crate::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
use crate::state_transition::StateTransitionEstimatedFeeValidation;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl StateTransitionEstimatedFeeValidation for ShieldFromAssetLockTransition {
    fn calculate_min_required_fee(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        // This value (the asset-lock base cost, `albc`) is now folded into the ShieldFromAssetLock
        // pool fee on the consensus path, so use `checked_mul` to match the rest of the
        // fully-checked shielded-fee arithmetic (overflow is unreachable with the current versioned
        // constant, but must never silently wrap).
        let asset_lock_base_cost = platform_version
            .dpp
            .state_transitions
            .identities
            .asset_locks
            .required_asset_lock_duff_balance_for_processing_start_for_address_funding
            .checked_mul(CREDITS_PER_DUFF)
            .ok_or(ProtocolError::Overflow(
                "asset_lock_base_cost credits conversion overflow",
            ))?;
        Ok(asset_lock_base_cost)
    }
}
