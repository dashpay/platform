use crate::address_funds::PlatformAddress;
use crate::shielded::SerializedAction;
use crate::state_transition::unshield_transition::methods::UnshieldTransitionMethodsV0;
use crate::state_transition::unshield_transition::v0::UnshieldTransitionV0;
use crate::{prelude::UserFeeIncrease, state_transition::StateTransition, ProtocolError};
use platform_version::version::PlatformVersion;

impl UnshieldTransitionMethodsV0 for UnshieldTransitionV0 {
    fn try_from_bundle(
        output_address: PlatformAddress,
        amount: u64,
        actions: Vec<SerializedAction>,
        flags: u8,
        value_balance: i64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        user_fee_increase: UserFeeIncrease,
        _platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        let transition = UnshieldTransitionV0 {
            output_address,
            amount,
            actions,
            flags,
            value_balance,
            anchor,
            proof,
            binding_signature,
            user_fee_increase,
        };
        Ok(transition.into())
    }
}
