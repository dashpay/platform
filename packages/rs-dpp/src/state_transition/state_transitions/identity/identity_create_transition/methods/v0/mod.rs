use crate::identity::signer::Signer;
use crate::identity::Identity;
use crate::prelude::AssetLockProof;
use crate::state_transition::StateTransitionType;
use crate::{BlsModule, ProtocolError};
use platform_version::version::PlatformVersion;

pub trait IdentityCreateTransitionMethodsV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity_with_signer<S: Signer>(
        identity: Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        signer: &S,
        bls: &impl BlsModule,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    /// Get State Transition type
    fn get_type() -> StateTransitionType;
}
