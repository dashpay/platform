mod v0;

use crate::prelude::IdentityNonce;
use crate::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use crate::voting::votes::Vote;
use platform_value::Identifier;
pub use v0::*;

impl MasternodeVoteTransitionAccessorsV0 for MasternodeVoteTransition {
    fn pro_tx_hash(&self) -> Identifier {
        match self {
            MasternodeVoteTransition::V0(transition) => transition.pro_tx_hash,
        }
    }

    fn voter_identity_id(&self) -> Identifier {
        match self {
            MasternodeVoteTransition::V0(transition) => transition.voter_identity_id,
        }
    }

    fn set_pro_tx_hash(&mut self, pro_tx_hash: Identifier) {
        match self {
            MasternodeVoteTransition::V0(transition) => {
                transition.pro_tx_hash = pro_tx_hash;
            }
        }
    }

    fn set_voter_identity_id(&mut self, voter_identity_id: Identifier) {
        match self {
            MasternodeVoteTransition::V0(transition) => {
                transition.voter_identity_id = voter_identity_id;
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

    fn nonce(&self) -> IdentityNonce {
        match self {
            MasternodeVoteTransition::V0(transition) => transition.nonce,
        }
    }
}
