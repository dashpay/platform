mod v0;

pub use v0::*;

#[cfg(feature = "state-transition-signing")]
use crate::identity::core_script::CoreScript;
#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
use crate::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
#[cfg(feature = "state-transition-signing")]
use crate::withdrawal::Pooling;
#[cfg(feature = "state-transition-signing")]
use crate::{
    prelude::UserFeeIncrease,
    state_transition::{
        shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0, StateTransition,
    },
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl ShieldedWithdrawalTransitionMethodsV0 for ShieldedWithdrawalTransition {
    #[cfg(feature = "state-transition-signing")]
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
    ) -> Result<StateTransition, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_serialization_versions
            .shielded_withdrawal_state_transition
            .default_current_version
        {
            0 => ShieldedWithdrawalTransitionV0::try_from_bundle(
                amount,
                actions,
                flags,
                value_balance,
                anchor,
                proof,
                binding_signature,
                core_fee_per_byte,
                pooling,
                output_script,
                user_fee_increase,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ShieldedWithdrawalTransition::try_from_bundle".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
