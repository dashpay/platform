mod v0;
pub use v0::*;

use crate::identity::signer::Signer;
use crate::identity::{Identity, IdentityPublicKey, KeyID};

use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
use crate::state_transition::identity_update_transition::IdentityUpdateTransition;

use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;
use crate::ProtocolError;

use platform_version::version::PlatformVersion;
use crate::prelude::IdentityNonce;

impl IdentityUpdateTransitionMethodsV0 for IdentityUpdateTransition {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity_with_signer<S: Signer>(
        identity: &Identity,
        master_public_key_id: &KeyID,
        add_public_keys: Vec<IdentityPublicKey>,
        disable_public_keys: Vec<KeyID>,
        public_keys_disabled_at: Option<u64>,
        nonce: IdentityNonce,
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
                public_keys_disabled_at,
                nonce,
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
