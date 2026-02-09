use std::collections::BTreeMap;

use crate::address_funds::PlatformAddress;
use crate::fee::Credits;
use crate::identity::state_transition::asset_lock_proof::AssetLockProof;

pub trait AddressFundingFromAssetLockTransitionAccessorsV0 {
    /// Get asset lock proof
    fn asset_lock_proof(&self) -> &AssetLockProof;
    /// Set asset lock proof
    fn set_asset_lock_proof(&mut self, asset_lock_proof: AssetLockProof);

    /// Get outputs (Some = explicit amount, None = remainder recipient)
    fn outputs(&self) -> &BTreeMap<PlatformAddress, Option<Credits>>;
    /// Get outputs as mutable
    fn outputs_mut(&mut self) -> &mut BTreeMap<PlatformAddress, Option<Credits>>;
    /// Set outputs
    fn set_outputs(&mut self, outputs: BTreeMap<PlatformAddress, Option<Credits>>);
}
