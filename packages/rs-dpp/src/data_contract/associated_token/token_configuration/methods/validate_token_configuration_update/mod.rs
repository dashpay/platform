use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::group::Group;
use crate::multi_identity_events::ActionTaker;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;

mod v0;

impl TokenConfiguration {
    pub fn validate_token_config_update(
        &self,
        new_config: &TokenConfiguration,
        contract_owner_id: &Identifier,
        main_group: &Group,
        action_taker: &ActionTaker,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .validation
            .data_contract
            .validate_token_config_update
        {
            0 => Ok(self.validate_token_config_update_v0(
                new_config,
                contract_owner_id,
                main_group,
                action_taker,
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "validate_token_config_update".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
