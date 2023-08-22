use platform_version::version::PlatformVersion;

use crate::identity::signer::Signer;
use crate::identity::{Identity, IdentityPublicKey};
use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;
use crate::{identity::KeyID, state_transition::StateTransitionType, ProtocolError};

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
