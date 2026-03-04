#[cfg(feature = "state-transition-signing")]
use crate::identity::core_script::CoreScript;
#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
use crate::state_transition::shielded_withdrawal_transition::methods::ShieldedWithdrawalTransitionMethodsV0;
use crate::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::withdrawal::Pooling;
#[cfg(feature = "state-transition-signing")]
use crate::{state_transition::StateTransition, ProtocolError};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl ShieldedWithdrawalTransitionMethodsV0 for ShieldedWithdrawalTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_bundle(
        amount: u64,
        actions: Vec<SerializedAction>,
        value_balance: i64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        core_fee_per_byte: u32,
        pooling: Pooling,
        output_script: CoreScript,
        _platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        let transition = ShieldedWithdrawalTransitionV0 {
            amount,
            actions,
            value_balance,
            anchor,
            proof,
            binding_signature,
            core_fee_per_byte,
            pooling,
            output_script,
        };
        Ok(transition.into())
    }
}
