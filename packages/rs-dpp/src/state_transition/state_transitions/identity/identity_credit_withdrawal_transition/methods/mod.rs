mod v0;

use crate::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use platform_version::version::FeatureVersion;
pub use v0::*;

use crate::identity::signer::Signer;
use crate::identity::Identity;

use crate::identity::core_script::CoreScript;
use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
use crate::state_transition::StateTransition;
use crate::version::PlatformVersion;
use crate::withdrawal::Pooling;
use crate::ProtocolError;

impl IdentityCreditWithdrawalTransitionMethodsV0 for IdentityCreditWithdrawalTransition {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity<S: Signer>(
        identity: &Identity,
        output_script: CoreScript,
        amount: u64,
        pooling: Pooling,
        core_fee_per_byte: u32,
        signer: S,
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
                output_script,
                amount,
                pooling,
                core_fee_per_byte,
                signer,
                platform_version,
                version,
            )?),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityCreditWithdrawalTransition version for try_from_identity {v}"
            ))),
        }
    }
}
