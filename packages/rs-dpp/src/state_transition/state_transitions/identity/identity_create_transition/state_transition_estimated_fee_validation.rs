use crate::balances::credits::CREDITS_PER_DUFF;
use crate::fee::Credits;
use crate::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use crate::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::state_transition::StateTransitionEstimatedFeeValidation;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl StateTransitionEstimatedFeeValidation for IdentityCreateTransition {
    fn calculate_min_required_fee(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        match platform_version
            .dpp
            .state_transitions
            .identities
            .calculate_min_required_fee_on_identity_create_transition
        {
            0 => Ok(self.calculate_min_required_fee_v0(platform_version)),
            1 => Ok(self.calculate_min_required_fee_v1(platform_version)),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown calculate_min_required_fee version for IdentityCreateTransition {v}"
            ))),
        }
    }
}

impl IdentityCreateTransition {
    fn calculate_min_required_fee_v0(&self, platform_version: &PlatformVersion) -> Credits {
        platform_version
            .dpp
            .state_transitions
            .identities
            .asset_locks
            .required_asset_lock_duff_balance_for_processing_start_for_identity_create
            * CREDITS_PER_DUFF
    }

    fn calculate_min_required_fee_v1(&self, platform_version: &PlatformVersion) -> Credits {
        let min_fees = &platform_version.fee_version.state_transition_min_fees;
        let base_cost = min_fees.identity_create_base_cost;
        let asset_lock_base_cost = platform_version
            .dpp
            .state_transitions
            .identities
            .asset_locks
            .required_asset_lock_duff_balance_for_processing_start_for_identity_create
            * CREDITS_PER_DUFF;
        let keys_in_creation = self.public_keys().len();
        base_cost.saturating_add(
            asset_lock_base_cost.saturating_add(
                min_fees
                    .identity_key_in_creation_cost
                    .saturating_mul(keys_in_creation as u64),
            ),
        )
    }
}
