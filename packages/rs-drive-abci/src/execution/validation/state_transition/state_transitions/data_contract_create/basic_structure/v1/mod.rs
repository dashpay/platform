use crate::error::Error;
use dpp::consensus::state::data_contract::data_contract_config_update_error::DataContractConfigUpdateError;
use dpp::dashcore::Network;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::{FeatureVersion, PlatformVersion};

use super::v0::DataContractCreateStateTransitionBasicStructureValidationV0;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_create) trait DataContractCreateStateTransitionBasicStructureValidationV1
{
    fn validate_basic_structure_v1(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DataContractCreateStateTransitionBasicStructureValidationV1 for DataContractCreateTransition {
    fn validate_basic_structure_v1(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // First run all v0 validations
        let v0_result = self.validate_basic_structure_v0(network_type, platform_version)?;
        if !v0_result.is_valid() {
            return Ok(v0_result);
        }

        // Validate config version meets minimum requirement for the current protocol version.
        // Since protocol version 12, V0 config is no longer accepted because it lacks
        // sized_integer_types support.
        let config_min_version = platform_version.dpp.contract_versions.config.min_version;
        if (self.data_contract().config().version() as FeatureVersion) < config_min_version {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                DataContractConfigUpdateError::new(
                    self.data_contract().id(),
                    format!(
                        "config version {} is not supported, minimum version is {}",
                        self.data_contract().config().version(),
                        config_min_version
                    ),
                )
                .into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
