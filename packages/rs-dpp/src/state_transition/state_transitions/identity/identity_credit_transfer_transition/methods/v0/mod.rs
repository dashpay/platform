use crate::identity::SecurityLevel;
use crate::state_transition::StateTransitionType;

pub trait IdentityCreditTransferTransitionMethodsV0 {
    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityCreditTransfer
    }
    fn security_level_requirement(&self) -> Vec<SecurityLevel>;
}
