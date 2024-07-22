mod v0;

use crate::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::FeatureVersion;
pub use v0::*;

#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::Identity;

#[cfg(feature = "state-transition-signing")]
use crate::identity::core_script::CoreScript;
#[cfg(feature = "state-transition-signing")]
use crate::identity::IdentityPublicKey;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::{IdentityNonce, UserFeeIncrease};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
#[cfg(feature = "state-transition-signing")]
use crate::version::PlatformVersion;
#[cfg(feature = "state-transition-signing")]
use crate::withdrawal::Pooling;
#[cfg(feature = "state-transition-signing")]
use crate::ProtocolError;

impl IdentityCreditWithdrawalTransitionMethodsV0 for IdentityCreditWithdrawalTransition {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity<S: Signer>(
        identity: &Identity,
        withdrawal_key_to_use: Option<&IdentityPublicKey>,
        output_script: CoreScript,
        amount: u64,
        pooling: Pooling,
        core_fee_per_byte: u32,
        user_fee_increase: UserFeeIncrease,
        signer: S,
        nonce: IdentityNonce,
        platform_version: &PlatformVersion,
        version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        match version.unwrap_or(
            platform_version
                .dpp
                .state_transition_conversion_versions
                .identity_to_identity_withdrawal_transition,
        ) {
            0 => Ok(IdentityCreditWithdrawalTransitionV0::try_from_identity(
                identity,
                withdrawal_key_to_use,
                output_script,
                amount,
                pooling,
                core_fee_per_byte,
                user_fee_increase,
                signer,
                nonce,
                platform_version,
                version,
            )?),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityCreditWithdrawalTransition version for try_from_identity {v}"
            ))),
        }
    }
}
