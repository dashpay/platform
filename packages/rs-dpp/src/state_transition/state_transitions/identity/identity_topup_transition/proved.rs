use crate::identity::state_transition::{AssetLockProved, OptionallyAssetLockProved};
use crate::prelude::AssetLockProof;
use crate::state_transition::identity_topup_transition::IdentityTopUpTransition;
use crate::ProtocolError;

impl OptionallyAssetLockProved for IdentityTopUpTransition {
    fn optional_asset_lock_proof(&self) -> Option<&AssetLockProof> {
        Some(self.asset_lock_proof())
    }
}

impl AssetLockProved for IdentityTopUpTransition {
    fn set_asset_lock_proof(
        &mut self,
        asset_lock_proof: AssetLockProof,
    ) -> Result<(), ProtocolError> {
        match self {
            Self::V0(v0) => v0.set_asset_lock_proof(asset_lock_proof),
        }
    }

    fn asset_lock_proof(&self) -> &AssetLockProof {
        match self {
            Self::V0(v0) => v0.asset_lock_proof(),
        }
    }
}
