use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use dpp::version::PlatformVersion;
use crate::error::Error;

pub(super) trait DocumentDeleteTransitionActionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error>;
}
impl DocumentDeleteTransitionActionStructureValidationV0 for DocumentDeleteTransitionAction {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error> {
        Ok(SimpleConsensusValidationResult::new())
    }
}
