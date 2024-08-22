use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::EpochIndex;
use dpp::state_transition::StateTransition;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;

mod v0;

pub trait ValidateTemporarilyDisabledContestedDocuments {
    fn validate_temporarily_disabled_contested_documents(
        &self,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<()>, Error>;
}

impl ValidateTemporarilyDisabledContestedDocuments for StateTransition {
    /// Disable contested document create transitions for the first 2 epochs
    fn validate_temporarily_disabled_contested_documents(
        &self,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<()>, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .validate_temporarily_disabled_contested_documents
        {
            0 => Ok(v0::validate_temporarily_disabled_contested_documents_v0(
                self, block_info,
            )),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "StateTransition::validate_temporary_disabled_contested_documents"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
