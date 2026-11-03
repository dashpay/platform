use crate::state_transition::identity_key_limits_update_transition::fields::*;
use crate::state_transition::identity_key_limits_update_transition::v0::IdentityKeyLimitsUpdateTransitionV0;
use crate::state_transition::StateTransitionFieldTypes;

impl StateTransitionFieldTypes for IdentityKeyLimitsUpdateTransitionV0 {
    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![IDENTITY_ID]
    }

    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, SIGNATURE_PUBLIC_KEY_ID]
    }
}
