use crate::state_transition::token_unshield_with_shielded_fee_transition::v0::TokenUnshieldWithShieldedFeeTransitionV0;
use crate::state_transition::StateTransitionFieldTypes;

impl StateTransitionFieldTypes for TokenUnshieldWithShieldedFeeTransitionV0 {
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
