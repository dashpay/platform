use crate::errors::ProtocolError;
use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::identity::state_transition::AssetLockProved;
use crate::state_transition::state_transitions::identity::identity_create_transition::v0::IdentityCreateTransitionV0;

impl AssetLockProved for IdentityCreateTransitionV0 {
    fn set_asset_lock_proof(
        &mut self,
        asset_lock_proof: AssetLockProof,
    ) -> Result<(), ProtocolError> {
        self.identity_id = asset_lock_proof.create_identifier()?;

        self.asset_lock_proof = asset_lock_proof;

        Ok(())
    }

    fn asset_lock_proof(&self) -> &AssetLockProof {
        &self.asset_lock_proof
    }
}
