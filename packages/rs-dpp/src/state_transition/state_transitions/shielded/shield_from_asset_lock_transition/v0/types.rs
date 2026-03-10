use crate::state_transition::shield_from_asset_lock_transition::fields::{PROOF, SIGNATURE};
use crate::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
use crate::state_transition::StateTransitionFieldTypes;

impl StateTransitionFieldTypes for ShieldFromAssetLockTransitionV0 {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, PROOF]
    }
}
