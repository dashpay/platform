use crate::identity::state_transition::AssetLockProved;
use crate::prelude::AssetLockProof;
use crate::state_transition::identity_topup_transition::IdentityTopUpTransition;
use crate::ProtocolError;

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
