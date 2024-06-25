use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use platform_value::BinaryData;

use crate::prelude::UserFeeIncrease;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

use crate::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
use crate::state_transition::masternode_vote_transition::MasternodeVoteTransition;

use crate::state_transition::StateTransition;
use crate::state_transition::StateTransitionType::MasternodeVote;
use crate::version::FeatureVersion;

impl From<MasternodeVoteTransitionV0> for StateTransition {
    fn from(value: MasternodeVoteTransitionV0) -> Self {
        let masternode_vote_transition: MasternodeVoteTransition = value.into();
        masternode_vote_transition.into()
    }
}

impl StateTransitionLike for MasternodeVoteTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        MasternodeVote
    }
    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData {
        &self.signature
    }
    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData) {
        self.signature = signature
    }

    fn user_fee_increase(&self) -> UserFeeIncrease {
        // The user fee increase for a masternode votes is always 0
        0
    }

    fn set_user_fee_increase(&mut self, _fee_multiplier: UserFeeIncrease) {
        // Setting does nothing
    }

    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.voter_identity_id]
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        self.signature = BinaryData::new(signature)
    }

    /// Get owner ID
    fn owner_id(&self) -> Identifier {
        self.voter_identity_id
    }

    fn unique_identifiers(&self) -> Vec<String> {
        vec![format!(
            "{}-{:x}",
            BASE64_STANDARD.encode(self.pro_tx_hash),
            self.nonce
        )]
    }
}
