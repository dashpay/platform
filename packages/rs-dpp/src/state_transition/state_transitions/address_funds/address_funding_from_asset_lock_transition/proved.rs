use crate::identity::state_transition::{AssetLockProved, OptionallyAssetLockProved};
use crate::prelude::AssetLockProof;
use crate::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
use crate::ProtocolError;

impl OptionallyAssetLockProved for AddressFundingFromAssetLockTransition {
    fn optional_asset_lock_proof(&self) -> Option<&AssetLockProof> {
        Some(self.asset_lock_proof())
    }
}

impl AssetLockProved for AddressFundingFromAssetLockTransition {
    fn set_asset_lock_proof(
        &mut self,
        asset_lock_proof: AssetLockProof,
    ) -> Result<(), ProtocolError> {
        match self {
            AddressFundingFromAssetLockTransition::V0(v0) => v0.set_asset_lock_proof(asset_lock_proof),
        }
    }

    fn asset_lock_proof(&self) -> &AssetLockProof {
        match self {
            AddressFundingFromAssetLockTransition::V0(v0) => v0.asset_lock_proof(),
        }
    }
}
