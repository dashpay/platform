use crate::data_contract::DataContract;
use crate::state_transition::documents_batch_transition::DocumentsBatchTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;

mod v0;

impl DocumentsBatchTransition {
    pub fn validate<'d>(
        &self,
        get_data_contract: impl Fn(Identifier) -> Result<Option<&'d DataContract>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .state_transitions
            .documents
            .documents_batch_transition
            .validation
            .validate
        {
            0 => self.validate_v0(get_data_contract, platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentsBatchTransition::validate".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
