use crate::state_transition::StateTransitionType;

pub trait MasternodeVoteTransitionMethodsV0 {
    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::MasternodeVote
    }
}
