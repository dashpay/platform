#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::{Identity, IdentityPublicKey};
#[cfg(feature = "state-transition-signing")]
use crate::prelude::{IdentityNonce, UserFeeIncrease};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
use crate::state_transition::StateTransitionType;
#[cfg(feature = "state-transition-signing")]
use crate::version::FeatureVersion;
#[cfg(feature = "state-transition-signing")]
use crate::{identity::KeyID, ProtocolError};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

pub trait IdentityUpdateTransitionMethodsV0 {
    /// Builds and signs an identity update that adds `add_public_keys` and disables
    /// `disable_public_keys`, signed by the MASTER key `master_public_key_id`.
    ///
    /// Refuses, before signing anything, an update that would disable the last valid
    /// authentication key of a security level, with [`ProtocolError::LastAuthenticationKey`]
    /// (DIP-11 signing-side policy; see
    /// [`check_keys_can_be_disabled`](crate::identity::key_disable_rule::check_keys_can_be_disabled)).
    /// The builder has no clock, so a key with any expiry does not count as remaining.
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    async fn try_from_identity_with_signer<S: Signer<IdentityPublicKey>>(
        identity: &Identity,
        master_public_key_id: &KeyID,
        add_public_keys: Vec<IdentityPublicKey>,
        disable_public_keys: Vec<KeyID>,
        nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError>;
    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityUpdate
    }
}
