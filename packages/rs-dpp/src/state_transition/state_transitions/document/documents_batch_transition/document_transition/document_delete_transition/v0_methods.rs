use platform_version::version::PlatformVersion;
use crate::data_contract::DataContract;
use crate::ProtocolError;
use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::documents_batch_transition::document_transition::document_delete_transition::v0::v0_methods::DocumentDeleteTransitionV0Methods;
use crate::state_transition::documents_batch_transition::document_transition::DocumentDeleteTransition;
use crate::validation::SimpleConsensusValidationResult;

impl DocumentDeleteTransitionV0Methods for DocumentDeleteTransition {
    fn base(&self) -> &DocumentBaseTransition {
        match self {
            DocumentDeleteTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut DocumentBaseTransition {
        match self {
            DocumentDeleteTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: DocumentBaseTransition) {
        match self {
            DocumentDeleteTransition::V0(v0) => v0.base = base,
        }
    }

    #[cfg(feature = "validation")]
    fn validate(
        &self,
        data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match self {
            DocumentDeleteTransition::V0(v0) => v0.validate(data_contract, platform_version),
        }
    }
}
