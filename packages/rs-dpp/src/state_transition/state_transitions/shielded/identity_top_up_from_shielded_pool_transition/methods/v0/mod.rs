#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
use crate::state_transition::StateTransitionType;
#[cfg(feature = "state-transition-signing")]
use crate::{state_transition::StateTransition, ProtocolError};
#[cfg(feature = "state-transition-signing")]
use platform_value::Identifier;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

pub trait IdentityTopUpFromShieldedPoolTransitionMethodsV0 {
    /// Build the transition from an already proven and signed Orchard spend bundle.
    /// The bundle's binding signature must have been computed over the platform
    /// sighash that binds `identity_id` and `top_up_amount` (see
    /// `identity_top_up_from_shielded_extra_sighash_data`).
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn try_from_bundle(
        identity_id: Identifier,
        actions: Vec<SerializedAction>,
        top_up_amount: u64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>;

    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityTopUpFromShieldedPool
    }
}
