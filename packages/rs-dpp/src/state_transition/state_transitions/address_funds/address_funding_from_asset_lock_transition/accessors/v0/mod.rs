use crate::fee::Credits;
use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::identity::KeyOfType;
use std::collections::BTreeMap;

pub trait AddressFundingFromAssetLockTransitionAccessorsV0 {
    /// Get asset lock proof
    fn asset_lock_proof(&self) -> &AssetLockProof;
    /// Set asset lock proof
    fn set_asset_lock_proof(&mut self, asset_lock_proof: AssetLockProof);

    /// Get outputs
    fn outputs(&self) -> &BTreeMap<KeyOfType, Credits>;
    /// Get outputs as mutable
    fn outputs_mut(&mut self) -> &mut BTreeMap<KeyOfType, Credits>;
    /// Set outputs
    fn set_outputs(&mut self, outputs: BTreeMap<KeyOfType, Credits>);

    /// Get the index of output paying fees
    fn output_paying_fees(&self) -> u16;
    /// Set the index of output paying fees
    fn set_output_paying_fees(&mut self, output_paying_fees: u16);
}
