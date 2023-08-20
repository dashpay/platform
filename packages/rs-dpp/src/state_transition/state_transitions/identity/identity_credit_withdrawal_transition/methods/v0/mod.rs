use crate::state_transition::StateTransitionType;

pub trait IdentityCreditWithdrawalTransitionMethodsV0 {
    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityCreditWithdrawal
    }
}
