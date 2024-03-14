mod v0;
pub use v0::*;

#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::{Identity, IdentityPublicKey, KeyID};

#[cfg(feature = "state-transition-signing")]
use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
use crate::state_transition::identity_update_transition::IdentityUpdateTransition;

#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
#[cfg(feature = "state-transition-signing")]
use crate::version::FeatureVersion;
#[cfg(feature = "state-transition-signing")]
use crate::ProtocolError;

#[cfg(feature = "state-transition-signing")]
use crate::prelude::{IdentityNonce, UserFeeIncrease};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl IdentityUpdateTransitionMethodsV0 for IdentityUpdateTransition {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity_with_signer<S: Signer>(
        identity: &Identity,
        master_public_key_id: &KeyID,
        add_public_keys: Vec<IdentityPublicKey>,
        disable_public_keys: Vec<KeyID>,
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
                .identity_update_state_transition
                .default_current_version,
        ) {
            0 => Ok(IdentityUpdateTransitionV0::try_from_identity_with_signer(
                identity,
                master_public_key_id,
                add_public_keys,
                disable_public_keys,
                nonce,
                user_fee_increase,
                signer,
                platform_version,
                version,
            )?),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityUpdateTransition version for try_from_identity_with_signer {v}"
            ))),
        }
    }
}
