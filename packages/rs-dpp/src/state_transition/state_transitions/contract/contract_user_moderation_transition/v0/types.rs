use crate::state_transition::contract_user_moderation_transition::fields::*;
use crate::state_transition::contract_user_moderation_transition::v0::ContractUserModerationTransitionV0;
use crate::state_transition::StateTransitionFieldTypes;

impl StateTransitionFieldTypes for ContractUserModerationTransitionV0 {
    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![OWNER_ID, DATA_CONTRACT_ID]
    }

    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, SIGNATURE_PUBLIC_KEY_ID]
    }
}
