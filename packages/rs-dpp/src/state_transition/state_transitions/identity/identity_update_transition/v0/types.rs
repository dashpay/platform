use crate::state_transition::identity_update_transition::fields::property_names::*;
use crate::state_transition::identity_update_transition::fields::*;
use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
use crate::state_transition::StateTransitionFieldTypes;

impl StateTransitionFieldTypes for IdentityUpdateTransitionV0 {
    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, ADD_PUBLIC_KEYS_SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![IDENTITY_ID]
    }

    fn signature_property_paths() -> Vec<&'static str> {
        vec![
            SIGNATURE,
            SIGNATURE_PUBLIC_KEY_ID,
            ADD_PUBLIC_KEYS_SIGNATURE,
        ]
    }
}
