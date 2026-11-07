use crate::fee::Credits;
use crate::state_transition::private_document_create_from_shielded_pool_transition::PrivateDocumentCreateFromShieldedPoolTransition;
use crate::state_transition::StateTransitionEstimatedFeeValidation;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl StateTransitionEstimatedFeeValidation for PrivateDocumentCreateFromShieldedPoolTransition {
    /// Pool-paid: the fee is the bundle's value balance, which the shielded minimum-fee
    /// validation holds to the exact fee, not an identity balance floor.
    fn calculate_min_required_fee(
        &self,
        _platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        Ok(0)
    }
}
