use crate::prelude::AssetLockProof;
use platform_value::Identifier;

pub trait IdentityTopUpTransitionAccessorsV0 {
    /// Set asset lock
    fn set_asset_lock_proof(&mut self, asset_lock_proof: AssetLockProof);
    /// Get asset lock proof
    fn asset_lock_proof(&self) -> &AssetLockProof;
    /// Set identity id
    fn set_identity_id(&mut self, identity_id: Identifier);
    /// Returns identity id
    fn identity_id(&self) -> &Identifier;
}
