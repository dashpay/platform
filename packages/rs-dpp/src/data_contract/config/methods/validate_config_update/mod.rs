use crate::data_contract::config::DataContractConfig;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;

mod v0;

impl DataContractConfig {
    pub fn validate_config_update(
        &self,
        new_config: &DataContractConfig,
        contract_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .validation
            .data_contract
            .validate_config_update
        {
            0 => Ok(self.validate_config_update_v0(new_config, contract_id)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "validate_config_update".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
