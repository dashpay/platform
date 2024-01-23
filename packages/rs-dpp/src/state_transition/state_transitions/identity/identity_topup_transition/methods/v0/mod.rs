use crate::identity::Identity;
use crate::prelude::AssetLockProof;
use crate::state_transition::{StateTransition, StateTransitionType};
use crate::{BlsModule, ProtocolError};
use platform_version::version::{FeatureVersion, PlatformVersion};

pub trait IdentityTopUpTransitionMethodsV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity(
        identity: Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        bls: &impl BlsModule,
        platform_version: &PlatformVersion,
        version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError>;

    /// Get State Transition type
    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityTopUp
    }

    /// Get asset lock minimal value
    fn get_minimal_asset_lock_value(
        platform_version: &PlatformVersion,
    ) -> Result<u64, ProtocolError>;
}
