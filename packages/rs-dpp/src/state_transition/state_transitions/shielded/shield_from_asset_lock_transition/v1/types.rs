use crate::state_transition::shield_from_asset_lock_transition::fields::{PROOF, SIGNATURE};
use crate::state_transition::shield_from_asset_lock_transition::v1::ShieldFromAssetLockTransitionV1;
use crate::state_transition::StateTransitionFieldTypes;

impl StateTransitionFieldTypes for ShieldFromAssetLockTransitionV1 {
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
