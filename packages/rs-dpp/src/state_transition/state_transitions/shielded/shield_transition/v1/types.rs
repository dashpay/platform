use crate::state_transition::shield_transition::v1::ShieldTransitionV1;
use crate::state_transition::StateTransitionFieldTypes;

impl StateTransitionFieldTypes for ShieldTransitionV1 {
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
