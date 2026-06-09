use crate::state_transition::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0;
use crate::state_transition::StateTransitionFieldTypes;

impl StateTransitionFieldTypes for IdentityCreateFromShieldedPoolTransitionV0 {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![]
    }
}
