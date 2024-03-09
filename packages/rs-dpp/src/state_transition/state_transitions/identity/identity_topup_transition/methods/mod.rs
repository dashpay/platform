mod v0;

pub use v0::*;

#[cfg(feature = "state-transition-signing")]
use crate::identity::Identity;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::{AssetLockProof, UserFeeIncrease};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
use crate::state_transition::identity_topup_transition::IdentityTopUpTransition;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
#[cfg(feature = "state-transition-signing")]
use crate::version::FeatureVersion;
#[cfg(feature = "state-transition-signing")]
use crate::ProtocolError;

#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl IdentityTopUpTransitionMethodsV0 for IdentityTopUpTransition {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity(
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
        version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        match version.unwrap_or(
            platform_version
                .dpp
                .state_transition_conversion_versions
                .identity_to_identity_top_up_transition,
        ) {
            0 => Ok(IdentityTopUpTransitionV0::try_from_identity(
                identity,
                asset_lock_proof,
                asset_lock_proof_private_key,
                user_fee_increase,
                platform_version,
                version,
            )?),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityTopUpTransition version for try_from_identity {v}"
            ))),
        }
    }
}
