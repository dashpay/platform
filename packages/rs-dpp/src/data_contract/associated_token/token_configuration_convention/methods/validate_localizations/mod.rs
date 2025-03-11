use crate::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;

impl TokenConfigurationConvention {
    pub fn validate_localizations(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .validation
            .data_contract
            .validate_localizations
        {
            0 => Ok(self.validate_localizations_v0()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "validate_localizations".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
