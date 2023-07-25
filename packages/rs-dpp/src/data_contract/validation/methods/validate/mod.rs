use crate::data_contract::DataContract;
use crate::validation::SimpleConsensusValidationResult;
use crate::version::PlatformVersion;
use crate::ProtocolError;

mod v0;

impl DataContract {
    /// Validate the data contract from a raw value
    pub fn validate(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version.dpp.validation.data_contract.validate {
            0 => self.validate_v0(platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::validate".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
