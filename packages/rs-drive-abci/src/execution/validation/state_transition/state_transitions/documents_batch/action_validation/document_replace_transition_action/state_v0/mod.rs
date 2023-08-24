use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::document::documents_batch::document_transition::document_replace_transition_action::DocumentReplaceTransitionAction;
use dpp::version::PlatformVersion;
use crate::error::Error;

pub(super) trait DocumentReplaceTransitionActionStateValidationV0 {
    fn validate_state_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl DocumentReplaceTransitionActionStateValidationV0 for DocumentReplaceTransitionAction {
    fn validate_state_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
    }
}
