mod v0;

use std::collections::BTreeMap;

use crate::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use crate::fee::Credits;
use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::prelude::AddressNonce;
use crate::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
pub use v0::*;

impl AddressFundingFromAssetLockTransitionAccessorsV0 for AddressFundingFromAssetLockTransition {
    fn asset_lock_proof(&self) -> &AssetLockProof {
        match self {
            AddressFundingFromAssetLockTransition::V0(v0) => &v0.asset_lock_proof,
        }
    }

    fn set_asset_lock_proof(&mut self, asset_lock_proof: AssetLockProof) {
        match self {
            AddressFundingFromAssetLockTransition::V0(v0) => v0.asset_lock_proof = asset_lock_proof,
        }
    }

    fn inputs(&self) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        match self {
            AddressFundingFromAssetLockTransition::V0(v0) => &v0.inputs,
        }
    }

    fn inputs_mut(&mut self) -> &mut BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        match self {
            AddressFundingFromAssetLockTransition::V0(v0) => &mut v0.inputs,
        }
    }

    fn set_inputs(&mut self, inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>) {
        match self {
            AddressFundingFromAssetLockTransition::V0(v0) => v0.inputs = inputs,
        }
    }

    fn outputs(&self) -> &BTreeMap<PlatformAddress, Credits> {
        match self {
            AddressFundingFromAssetLockTransition::V0(v0) => &v0.outputs,
        }
    }

    fn outputs_mut(&mut self) -> &mut BTreeMap<PlatformAddress, Credits> {
        match self {
            AddressFundingFromAssetLockTransition::V0(v0) => &mut v0.outputs,
        }
    }

    fn set_outputs(&mut self, outputs: BTreeMap<PlatformAddress, Credits>) {
        match self {
            AddressFundingFromAssetLockTransition::V0(v0) => v0.outputs = outputs,
        }
    }

    fn fee_strategy(&self) -> &AddressFundsFeeStrategy {
        match self {
            AddressFundingFromAssetLockTransition::V0(v0) => &v0.fee_strategy,
        }
    }

    fn set_fee_strategy(&mut self, fee_strategy: AddressFundsFeeStrategy) {
        match self {
            AddressFundingFromAssetLockTransition::V0(v0) => v0.fee_strategy = fee_strategy,
        }
    }
}
