use crate::state_transition_action::identity::masternode_vote::v0::MasternodeVoteTransitionActionV0;
use dpp::state_transition::state_transitions::identity::masternode_vote_transition::v0::MasternodeVoteTransitionV0;

impl From<MasternodeVoteTransitionV0> for MasternodeVoteTransitionActionV0 {
    fn from(value: MasternodeVoteTransitionV0) -> Self {
        let MasternodeVoteTransitionV0 {
            pro_tx_hash,
            vote,
            ..
        } = value;
        MasternodeVoteTransitionActionV0 {
            pro_tx_hash,
            vote,
        }
    }
}

impl From<&MasternodeVoteTransitionV0> for MasternodeVoteTransitionActionV0 {
    fn from(value: &MasternodeVoteTransitionV0) -> Self {
        let MasternodeVoteTransitionV0 {
            pro_tx_hash,
            vote,
            ..
        } = value;
        MasternodeVoteTransitionActionV0 {
            pro_tx_hash: *pro_tx_hash,
            vote: vote.clone(),
        }
    }
}
