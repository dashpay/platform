use crate::identity::Identity;
use crate::prelude::AssetLockProof;
use crate::state_transition::StateTransitionType;
use crate::{BlsModule, ProtocolError};
use platform_version::version::FeatureVersion;

pub trait IdentityTopUpTransitionMethodsV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity(
        identity: Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        bls: &impl BlsModule,
        version: FeatureVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;

    /// Get State Transition type
    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityTopUp
    }
}
