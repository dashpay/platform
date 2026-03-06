#[cfg(feature = "state-transition-signing")]
use crate::address_funds::PlatformAddress;
#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
use crate::state_transition::unshield_transition::methods::UnshieldTransitionMethodsV0;
use crate::state_transition::unshield_transition::v0::UnshieldTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::{state_transition::StateTransition, ProtocolError};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl UnshieldTransitionMethodsV0 for UnshieldTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_bundle(
        output_address: PlatformAddress,
        actions: Vec<SerializedAction>,
        unshielding_amount: u64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        _platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        let transition = UnshieldTransitionV0 {
            output_address,
            actions,
            unshielding_amount,
            anchor,
            proof,
            binding_signature,
        };
        Ok(transition.into())
    }
}
