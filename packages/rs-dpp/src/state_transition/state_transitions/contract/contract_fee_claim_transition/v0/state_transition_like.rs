use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use platform_value::BinaryData;

use crate::prelude::UserFeeIncrease;
use crate::state_transition::StateTransitionHasUserFeeIncrease;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionOwned, StateTransitionType},
};

use crate::state_transition::contract_fee_claim_transition::v0::ContractFeeClaimTransitionV0;
use crate::state_transition::contract_fee_claim_transition::ContractFeeClaimTransition;

use crate::state_transition::StateTransitionType::ContractFeeClaim;
use crate::state_transition::{StateTransition, StateTransitionSingleSigned};
use crate::version::FeatureVersion;

impl From<ContractFeeClaimTransitionV0> for StateTransition {
    fn from(value: ContractFeeClaimTransitionV0) -> Self {
        let transition: ContractFeeClaimTransition = value.into();
        transition.into()
    }
}

impl StateTransitionLike for ContractFeeClaimTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        ContractFeeClaim
    }

    /// Returns the ID of the contract whose pot is paid out
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.data_contract_id]
    }

    /// Unique per signer, contract and contract nonce, as a contract update is.
    fn unique_identifiers(&self) -> Vec<String> {
        vec![format!(
            "{}-{}-{:x}",
            BASE64_STANDARD.encode(self.owner_id),
            BASE64_STANDARD.encode(self.data_contract_id),
            self.identity_contract_nonce
        )]
    }
}

impl StateTransitionHasUserFeeIncrease for ContractFeeClaimTransitionV0 {
    fn user_fee_increase(&self) -> UserFeeIncrease {
        self.user_fee_increase
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.user_fee_increase = user_fee_increase
    }
}

impl StateTransitionSingleSigned for ContractFeeClaimTransitionV0 {
    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData {
        &self.signature
    }

    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData) {
        self.signature = signature
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        self.signature = BinaryData::new(signature)
    }
}

impl StateTransitionOwned for ContractFeeClaimTransitionV0 {
    /// Get owner ID
    fn owner_id(&self) -> Identifier {
        self.owner_id
    }
}
