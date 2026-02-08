use crate::data_contract::config::DataContractConfig;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;

mod v0;
mod v1;

impl DataContractConfig {
    pub fn validate_update(
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
            0 => Ok(self.validate_update_v0(new_config, contract_id)),
            1 => Ok(self.validate_update_v1(new_config, contract_id)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "validate_update".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}
