use crate::error::Error;

use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;

use dpp::validation::SimpleConsensusValidationResult;

use dpp::version::PlatformVersion;
use drive::state_transition_action::document::documents_batch::DocumentsBatchTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::documents_batch) trait DocumentsBatchStateTransitionStructureValidationV0
{
    fn validate_structure_v0(
        &self,
        action: &DocumentsBatchTransitionAction,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentsBatchStateTransitionStructureValidationV0 for DocumentsBatchTransition {
    fn validate_structure_v0(
        &self,
        action: &DocumentsBatchTransitionAction,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // First we should validate the base structure
        self.validate_base_structure(platform_version)
            .map_err(Error::Protocol)
    }
}
