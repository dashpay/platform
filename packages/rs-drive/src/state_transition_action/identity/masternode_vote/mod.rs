/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::identity::masternode_vote::v0::MasternodeVoteTransitionActionV0;
use derive_more::From;
use dpp::platform_value::Identifier;
use dpp::voting::resource_vote::ResourceVote;

/// action
#[derive(Debug, Clone, From)]
pub enum MasternodeVoteTransitionAction {
    /// v0
    V0(MasternodeVoteTransitionActionV0),
}

impl MasternodeVoteTransitionAction {
    /// the pro tx hash identifier of the masternode
    pub fn pro_tx_hash(&self) -> Identifier {
        match self {
            MasternodeVoteTransitionAction::V0(transition) => transition.pro_tx_hash,
        }
    }

    /// Resource vote
    pub fn resource_vote(&self) -> ResourceVote {
        match self {
            MasternodeVoteTransitionAction::V0(transition) => transition.resource_vote,
        }
    }
}
