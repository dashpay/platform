use std::collections::BTreeMap;

use crate::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use crate::fee::Credits;
use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::prelude::AddressNonce;

pub trait AddressFundingFromAssetLockTransitionAccessorsV0 {
    /// Get asset lock proof
    fn asset_lock_proof(&self) -> &AssetLockProof;
    /// Set asset lock proof
    fn set_asset_lock_proof(&mut self, asset_lock_proof: AssetLockProof);

    /// Get inputs (existing platform addresses to combine funds from)
    fn inputs(&self) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)>;
    /// Get inputs as mutable
    fn inputs_mut(&mut self) -> &mut BTreeMap<PlatformAddress, (AddressNonce, Credits)>;
    /// Set inputs
    fn set_inputs(&mut self, inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>);

    /// Get outputs (Some = explicit amount, None = remainder recipient)
    fn outputs(&self) -> &BTreeMap<PlatformAddress, Option<Credits>>;
    /// Get outputs as mutable
    fn outputs_mut(&mut self) -> &mut BTreeMap<PlatformAddress, Option<Credits>>;
    /// Set outputs
    fn set_outputs(&mut self, outputs: BTreeMap<PlatformAddress, Option<Credits>>);

    /// Get fee strategy
    fn fee_strategy(&self) -> &AddressFundsFeeStrategy;
    /// Set fee strategy
    fn set_fee_strategy(&mut self, fee_strategy: AddressFundsFeeStrategy);
}
