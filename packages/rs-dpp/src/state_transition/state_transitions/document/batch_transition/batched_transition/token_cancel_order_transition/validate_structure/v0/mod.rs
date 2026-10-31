use crate::state_transition::batch_transition::TokenCancelOrderTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

pub(super) trait TokenCancelOrderTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl TokenCancelOrderTransitionStructureValidationV0 for TokenCancelOrderTransition {
    /// A cancel names an order and its side, which state validation finds and checks.
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        Ok(SimpleConsensusValidationResult::default())
    }
}
