mod v0;

pub use v0::*;

#[cfg(feature = "state-transition-signing")]
use crate::identity::Identity;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::{AssetLockProof, UserFeeIncrease};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;
use crate::state_transition::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
#[cfg(feature = "state-transition-signing")]
use crate::version::FeatureVersion;
#[cfg(feature = "state-transition-signing")]
use crate::ProtocolError;

#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl IdentityTopUpFromAddressesTransitionMethodsV0 for IdentityTopUpFromAddressesTransition {
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
                .identity_to_identity_top_up_from_addresses_transition,
        ) {
            0 => Ok(IdentityTopUpFromAddressesTransitionV0::try_from_identity(
                identity,
                asset_lock_proof,
                asset_lock_proof_private_key,
                user_fee_increase,
                platform_version,
                version,
            )?),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityTopUpFromAddressesTransition version for try_from_identity {v}"
            ))),
        }
    }
}
