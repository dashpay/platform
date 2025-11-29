mod v0;

use std::collections::BTreeMap;

use crate::address_funds::PlatformAddress;
use crate::fee::Credits;
use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
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

    fn output_paying_fees(&self) -> u16 {
        match self {
            AddressFundingFromAssetLockTransition::V0(v0) => v0.output_paying_fees,
        }
    }

    fn set_output_paying_fees(&mut self, output_paying_fees: u16) {
        match self {
            AddressFundingFromAssetLockTransition::V0(v0) => {
                v0.output_paying_fees = output_paying_fees
            }
        }
    }
}
