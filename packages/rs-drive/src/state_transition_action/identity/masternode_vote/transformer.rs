use crate::state_transition_action::identity::masternode_vote::v0::MasternodeVoteTransitionActionV0;
use crate::state_transition_action::identity::masternode_vote::MasternodeVoteTransitionAction;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;

impl From<MasternodeVoteTransition> for MasternodeVoteTransitionAction {
    fn from(value: MasternodeVoteTransition) -> Self {
        match value {
            MasternodeVoteTransition::V0(v0) => MasternodeVoteTransitionActionV0::from(v0).into(),
        }
    }
}

impl From<&MasternodeVoteTransition> for MasternodeVoteTransitionAction {
    fn from(value: &MasternodeVoteTransition) -> Self {
        match value {
            MasternodeVoteTransition::V0(v0) => MasternodeVoteTransitionActionV0::from(v0).into(),
        }
    }
}
