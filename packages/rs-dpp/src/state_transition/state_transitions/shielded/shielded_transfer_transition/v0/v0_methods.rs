#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
use crate::state_transition::shielded_transfer_transition::methods::ShieldedTransferTransitionMethodsV0;
use crate::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::{state_transition::StateTransition, ProtocolError};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl ShieldedTransferTransitionMethodsV0 for ShieldedTransferTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_bundle(
        actions: Vec<SerializedAction>,
        value_balance: u64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        _platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        let transition = ShieldedTransferTransitionV0 {
            actions,
            value_balance,
            anchor,
            proof,
            binding_signature,
        };
        Ok(transition.into())
    }
}
