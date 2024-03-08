use crate::identity::Identity;
use crate::prelude::{AssetLockProof, UserFeeIncrease};
use crate::state_transition::{StateTransition, StateTransitionType};
use crate::ProtocolError;
use platform_version::version::{FeatureVersion, PlatformVersion};

pub trait IdentityTopUpTransitionMethodsV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity(
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
        version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError>;

    /// Get State Transition type
    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityTopUp
    }
}
