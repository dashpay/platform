mod v0;

use crate::prelude::IdentityNonce;
use crate::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use crate::voting::votes::Vote;
use platform_value::Identifier;
pub use v0::*;

impl MasternodeVoteTransitionAccessorsV0 for MasternodeVoteTransition {
    fn nonce(&self) -> IdentityNonce {
        match self {
            MasternodeVoteTransition::V0(transition) => transition.nonce,
        }
    }

    fn pro_tx_hash(&self) -> Identifier {
        match self {
            MasternodeVoteTransition::V0(transition) => transition.pro_tx_hash,
        }
    }

    fn set_pro_tx_hash(&mut self, pro_tx_hash: Identifier) {
        match self {
            MasternodeVoteTransition::V0(transition) => {
                transition.pro_tx_hash = pro_tx_hash;
            }
        }
    }

    fn vote(&self) -> &Vote {
        match self {
            MasternodeVoteTransition::V0(transition) => &transition.vote,
        }
    }

    fn set_vote(&mut self, vote: Vote) {
        match self {
            MasternodeVoteTransition::V0(transition) => {
                transition.vote = vote;
            }
        }
    }
}
