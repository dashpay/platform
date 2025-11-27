use crate::state_transition::address_funding_from_asset_lock_transition::fields::*;
use crate::state_transition::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
use crate::state_transition::StateTransitionFieldTypes;

impl StateTransitionFieldTypes for AddressFundingFromAssetLockTransitionV0 {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, PUBLIC_KEYS_SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![IDENTITY_ID]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![]
    }
}
