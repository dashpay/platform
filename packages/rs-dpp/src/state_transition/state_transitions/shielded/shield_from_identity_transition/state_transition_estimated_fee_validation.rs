use crate::consensus::state::identity::IdentityInsufficientBalanceError;
use crate::fee::Credits;
use crate::shielded::compute_shielded_identity_balance_write_fee;
use crate::state_transition::shield_from_identity_transition::accessors::ShieldFromIdentityTransitionAccessorsV0;
use crate::state_transition::shield_from_identity_transition::ShieldFromIdentityTransition;
use crate::state_transition::{
    StateTransitionEstimatedFeeValidation, StateTransitionIdentityEstimatedFeeValidation,
};
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl StateTransitionEstimatedFeeValidation for ShieldFromIdentityTransition {
    /// The stateless floor is the conservative complete fee: the shielded compute fee,
    /// the per-action note storage allowance, and the flat identity balance write
    /// allowance (`compute_shielded_identity_balance_write_fee`). Storage and processing
    /// of the note and identity writes are metered by GroveDB, so the authoritative
    /// funding gate is still the identity-paid fee validation of the execution event;
    /// this floor exists so an identity that could not pay the complete fee is refused
    /// before the expensive Orchard proof verification runs.
    fn calculate_min_required_fee(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        compute_shielded_identity_balance_write_fee(self.actions().len(), platform_version)
    }
}

impl StateTransitionIdentityEstimatedFeeValidation for ShieldFromIdentityTransition {
    fn validate_estimated_fee(
        &self,
        identity_known_balance: Credits,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let required_fee = self.calculate_min_required_fee(platform_version)?;
        let outbound_amount = self.amount().saturating_add(required_fee);

        if identity_known_balance < outbound_amount {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                IdentityInsufficientBalanceError::new(
                    self.identity_id(),
                    identity_known_balance,
                    outbound_amount,
                )
                .into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
