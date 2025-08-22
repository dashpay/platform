#[cfg(feature = "state-transition-signing")]
use crate::{
    identity::{core_script::CoreScript, signer::Signer, Identity, IdentityPublicKey},
    prelude::{IdentityNonce, UserFeeIncrease},
    state_transition::StateTransition,
    withdrawal::Pooling,
    ProtocolError,
};

use crate::state_transition::StateTransitionType;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::{FeatureVersion, PlatformVersion};

/// The key purpose that is preferred for signing the withdrawal
#[cfg(feature = "state-transition-signing")]
pub enum PreferredKeyPurposeForSigningWithdrawal {
    /// Use any key
    Any,
    /// Use the owner key, then the transfer key
    OwnerPreferred,
    /// Use the transfer key, then the owner key
    TransferPreferred,
    /// Only use the owner key
    OwnerOnly,
    /// Only use the transfer key
    TransferOnly,
}

pub trait IdentityCreditWithdrawalTransitionMethodsV0 {
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn try_from_identity<S: Signer>(
        identity: &Identity,
        output_script: Option<CoreScript>,
        amount: u64,
        pooling: Pooling,
        core_fee_per_byte: u32,
        user_fee_increase: UserFeeIncrease,
        signer: S,
        signing_withdrawal_key_to_use: Option<&IdentityPublicKey>,
        preferred_key_purpose_for_signing_withdrawal: PreferredKeyPurposeForSigningWithdrawal,
        nonce: IdentityNonce,
        platform_version: &PlatformVersion,
        version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError>;

    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityCreditWithdrawal
    }
}
