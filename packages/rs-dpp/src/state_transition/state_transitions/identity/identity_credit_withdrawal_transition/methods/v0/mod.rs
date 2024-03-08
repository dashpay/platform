use crate::identity::core_script::CoreScript;
use crate::identity::signer::Signer;
use crate::identity::Identity;
use crate::prelude::{UserFeeIncrease, IdentityNonce};
use crate::state_transition::{StateTransition, StateTransitionType};
use crate::withdrawal::Pooling;
use crate::ProtocolError;
use platform_version::version::{FeatureVersion, PlatformVersion};

pub trait IdentityCreditWithdrawalTransitionMethodsV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity<S: Signer>(
        identity: &Identity,
        output_script: CoreScript,
        amount: u64,
        pooling: Pooling,
        core_fee_per_byte: u32,
        user_fee_increase: UserFeeIncrease,
        signer: S,
        nonce: IdentityNonce,
        _platform_version: &PlatformVersion,
        _version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError>;

    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityCreditWithdrawal
    }
}
