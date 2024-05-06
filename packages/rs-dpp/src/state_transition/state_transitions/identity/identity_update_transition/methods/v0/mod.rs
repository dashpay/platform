use platform_version::version::PlatformVersion;

use crate::identity::signer::Signer;
use crate::identity::Identity;
use crate::identity::identity_public_key::IdentityPublicKey;
use crate::state_transition::StateTransition;
use platform_version::version::FeatureVersion;
use crate::{identity::identity_public_key::KeyID, state_transition::StateTransitionType, errors::ProtocolError};

pub trait IdentityUpdateTransitionMethodsV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity_with_signer<S: Signer>(
        identity: &Identity,
        master_public_key_id: &KeyID,
        add_public_keys: Vec<IdentityPublicKey>,
        disable_public_keys: Vec<KeyID>,
        public_keys_disabled_at: Option<u64>,
        signer: &S,
        platform_version: &PlatformVersion,
        version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError>;
    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityUpdate
    }
}
