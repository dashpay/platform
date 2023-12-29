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

/// Only state transitions funded with Asset Lock Transactions have Asset Lock Proofs
pub trait OptionallyAssetLockProved {
    /// Returns asset lock proof if state transition implements it
    fn optional_asset_lock_proof(&self) -> Option<&AssetLockProof> {
        None
    }
}
