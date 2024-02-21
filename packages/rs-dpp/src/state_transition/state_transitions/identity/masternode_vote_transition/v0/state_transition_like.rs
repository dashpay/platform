use platform_value::BinaryData;

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
    /// Returns ID of the created contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.pro_tx_hash]
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        self.signature = BinaryData::new(signature)
    }

    /// Get owner ID
    fn owner_id(&self) -> Identifier {
        self.pro_tx_hash
    }

    fn unique_identifiers(&self) -> Vec<String> {
        vec![format!(
            "{}-{:x}",
            base64::encode(self.pro_tx_hash),
            self.nonce
        )]
    }
}
