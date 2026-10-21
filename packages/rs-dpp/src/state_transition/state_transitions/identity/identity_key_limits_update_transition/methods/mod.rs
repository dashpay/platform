mod v0;
pub use v0::*;

#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::{Identity, IdentityPublicKey, KeyID, TimestampMillis};
#[cfg(feature = "state-transition-signing")]
use crate::prelude::{IdentityNonce, UserFeeIncrease};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::identity_key_limits_update_transition::v0::IdentityKeyLimitsUpdateTransitionV0;
use crate::state_transition::identity_key_limits_update_transition::IdentityKeyLimitsUpdateTransition;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
#[cfg(feature = "state-transition-signing")]
use crate::version::FeatureVersion;
#[cfg(feature = "state-transition-signing")]
use crate::ProtocolError;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl IdentityKeyLimitsUpdateTransitionMethodsV0 for IdentityKeyLimitsUpdateTransition {
    #[cfg(feature = "state-transition-signing")]
    async fn try_from_identity_with_signer<S: Signer<IdentityPublicKey>>(
        identity: &Identity,
        signing_key_id: &KeyID,
        key_id: KeyID,
        total_budget: Option<Credits>,
        expires_at: Option<TimestampMillis>,
        nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        match version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .identity_key_limits_update_state_transition
                .default_current_version,
        ) {
            0 => Ok(
                IdentityKeyLimitsUpdateTransitionV0::try_from_identity_with_signer(
                    identity,
                    signing_key_id,
                    key_id,
                    total_budget,
                    expires_at,
                    nonce,
                    user_fee_increase,
                    signer,
                    platform_version,
                    version,
                )
                .await?,
            ),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityKeyLimitsUpdateTransition version for try_from_identity_with_signer {v}"
            ))),
        }
    }
}
