// TODO: Move to state_transitions module

use crate::ProtocolError;
use asset_lock_proof::AssetLockProof;

pub mod asset_lock_proof;

/// Objects with Asset Lock Proof
pub trait AssetLockProved {
    /// Set asset lock proof
    fn set_asset_lock_proof(
        &mut self,
        asset_lock_proof: AssetLockProof,
    ) -> Result<(), ProtocolError>;

    /// Get asset lock proof
    fn asset_lock_proof(&self) -> &AssetLockProof;
}
