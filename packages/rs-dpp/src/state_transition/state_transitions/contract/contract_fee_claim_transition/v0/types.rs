use crate::state_transition::contract_fee_claim_transition::fields::*;
use crate::state_transition::contract_fee_claim_transition::v0::ContractFeeClaimTransitionV0;
use crate::state_transition::StateTransitionFieldTypes;

impl StateTransitionFieldTypes for ContractFeeClaimTransitionV0 {
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
