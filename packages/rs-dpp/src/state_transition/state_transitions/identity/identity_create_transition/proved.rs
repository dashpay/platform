use crate::identity::state_transition::AssetLockProved;
use crate::prelude::AssetLockProof;
use crate::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::ProtocolError;

impl AssetLockProved for IdentityCreateTransition {
    fn set_asset_lock_proof(
        &mut self,
        asset_lock_proof: AssetLockProof,
    ) -> Result<(), ProtocolError> {
        match self {
            IdentityCreateTransition::V0(v0) => v0.set_asset_lock_proof(asset_lock_proof),
        }
    }

    fn asset_lock_proof(&self) -> &AssetLockProof {
        match self {
            IdentityCreateTransition::V0(v0) => v0.asset_lock_proof(),
        }
    }
}
