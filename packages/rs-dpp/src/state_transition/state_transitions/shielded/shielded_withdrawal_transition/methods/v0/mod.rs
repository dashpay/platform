use crate::identity::core_script::CoreScript;
use crate::shielded::SerializedAction;
use crate::state_transition::StateTransitionType;
use crate::withdrawal::Pooling;
use crate::{
    prelude::UserFeeIncrease,
    state_transition::StateTransition,
    ProtocolError,
};
use platform_version::version::PlatformVersion;

pub trait ShieldedWithdrawalTransitionMethodsV0 {
    #[allow(clippy::too_many_arguments)]
    fn try_from_bundle(
        amount: u64,
        actions: Vec<SerializedAction>,
        flags: u8,
        value_balance: i64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        core_fee_per_byte: u32,
        pooling: Pooling,
        output_script: CoreScript,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>;

    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::ShieldedWithdrawal
    }
}
