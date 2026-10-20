#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
use crate::state_transition::identity_top_up_from_shielded_pool_transition::methods::IdentityTopUpFromShieldedPoolTransitionMethodsV0;
use crate::state_transition::identity_top_up_from_shielded_pool_transition::v0::IdentityTopUpFromShieldedPoolTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::{state_transition::StateTransition, ProtocolError};
#[cfg(feature = "state-transition-signing")]
use platform_value::Identifier;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl IdentityTopUpFromShieldedPoolTransitionMethodsV0
    for IdentityTopUpFromShieldedPoolTransitionV0
{
    #[cfg(feature = "state-transition-signing")]
    fn try_from_bundle(
        identity_id: Identifier,
        actions: Vec<SerializedAction>,
        top_up_amount: u64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        _platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        let transition = IdentityTopUpFromShieldedPoolTransitionV0 {
            identity_id,
            actions,
            top_up_amount,
            anchor,
            proof,
            binding_signature,
        };
        Ok(transition.into())
    }
}
