mod v0;

use crate::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use crate::voting::resource_vote::ResourceVote;
use platform_value::Identifier;
pub use v0::*;

impl MasternodeVoteTransitionAccessorsV0 for MasternodeVoteTransition {
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

    fn resource_vote(&self) -> ResourceVote {
        match self {
            MasternodeVoteTransition::V0(transition) => transition.resource_vote,
        }
    }

    fn set_resource_vote(&mut self, resource_vote: ResourceVote) {
        match self {
            MasternodeVoteTransition::V0(transition) => {
                transition.resource_vote = resource_vote;
            }
        }
    }
}
