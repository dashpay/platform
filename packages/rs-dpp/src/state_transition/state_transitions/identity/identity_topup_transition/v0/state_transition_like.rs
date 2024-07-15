use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use platform_value::BinaryData;

use crate::prelude::UserFeeIncrease;
use crate::state_transition::identity_topup_transition::IdentityTopUpTransition;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;

use crate::state_transition::StateTransition;
use crate::state_transition::StateTransitionType::IdentityTopUp;
use crate::version::FeatureVersion;

impl From<IdentityTopUpTransitionV0> for StateTransition {
    fn from(value: IdentityTopUpTransitionV0) -> Self {
        let transition: IdentityTopUpTransition = value.into();
        transition.into()
    }
}

impl StateTransitionLike for IdentityTopUpTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        IdentityTopUp
    }
    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData {
        &self.signature
    }
    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData) {
        self.signature = signature
    }
    /// Returns ID of the topUpd contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.identity_id]
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        self.signature = BinaryData::new(signature)
    }

    /// Get owner ID
    fn owner_id(&self) -> Identifier {
        self.identity_id
    }

    /// We want transactions to be unique based on the asset lock proof, here there is a
    /// conflict on purpose with identity create transitions
    fn unique_identifiers(&self) -> Vec<String> {
        let identifier = self.asset_lock_proof.create_identifier();
        match identifier {
            Ok(identifier) => {
                vec![BASE64_STANDARD.encode(identifier)]
            }
            Err(_) => {
                // no unique identifier, this won't actually occur on Platform
                // as we ask for the unique identifier after validation
                vec![String::default()]
            }
        }
    }

    fn user_fee_increase(&self) -> UserFeeIncrease {
        self.user_fee_increase
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.user_fee_increase = user_fee_increase
    }
}
