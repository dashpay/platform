use crate::identity::identity_public_key::{Purpose, SecurityLevel, SecurityLevel::{CRITICAL, HIGH, MEDIUM}};
use crate::identity::KeyID;
use crate::state_transition::identity::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
use crate::state_transition::StateTransitionIdentitySigned;

impl StateTransitionIdentitySigned for MasternodeVoteTransitionV0 {
    fn signature_public_key_id(&self) -> KeyID {
        self.signature_public_key_id
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        self.signature_public_key_id = key_id
    }

    fn security_level_requirement(&self) -> Vec<SecurityLevel> {
        vec![CRITICAL, HIGH, MEDIUM]
    }

    fn purpose_requirement(&self) -> Purpose {
        Purpose::VOTING
    }
}
