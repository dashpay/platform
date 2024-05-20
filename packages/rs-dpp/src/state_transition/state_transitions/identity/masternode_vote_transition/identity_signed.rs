use crate::identity::{KeyID, Purpose, SecurityLevel};
use crate::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use crate::state_transition::StateTransitionIdentitySigned;

impl StateTransitionIdentitySigned for MasternodeVoteTransition {
    fn signature_public_key_id(&self) -> KeyID {
        match self {
            MasternodeVoteTransition::V0(transition) => transition.signature_public_key_id(),
        }
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        match self {
            MasternodeVoteTransition::V0(transition) => {
                transition.set_signature_public_key_id(key_id)
            }
        }
    }

    fn security_level_requirement(&self) -> Vec<SecurityLevel> {
        match self {
            MasternodeVoteTransition::V0(transition) => transition.security_level_requirement(),
        }
    }

    fn purpose_requirement(&self) -> Purpose {
        match self {
            MasternodeVoteTransition::V0(transition) => transition.purpose_requirement(),
        }
    }
}
